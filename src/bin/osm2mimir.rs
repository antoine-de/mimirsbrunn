// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use failure::format_err;
use futures::stream::StreamExt;
use mimir2::{
    adapters::secondary::elasticsearch::{
        self,
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
    },
    domain::model::{configuration::Configuration, document::Document, index::IndexVisibility},
    domain::ports::{list::ListParameters, remote::Remote},
    domain::usecases::{
        generate_index::{GenerateIndex, GenerateIndexParameters},
        list_documents::{ListDocuments, ListDocumentsParameters},
        search_documents::SearchDocuments,
        UseCase,
    },
};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::osm_reader::make_osm_reader;
use mimirsbrunn::osm_reader::poi::{add_address, compute_weight, pois, PoiConfig};
use mimirsbrunn::osm_reader::street::{compute_street_weight, streets};
use mimirsbrunn::settings::osm2mimir::{Args, Settings};
use places::{admin::Admin, poi::Poi, street::Street, MimirObject};
use serde::Serialize;
use slog_scope::{debug, info};

async fn run(args: Args) -> Result<(), mimirsbrunn::Error> {
    let input = args.input.clone(); // we save the input, because args will be consumed by settings.
    validate_args(&args)?;
    let settings = Settings::new(args)?;

    let mut osm_reader = make_osm_reader(&input)?;
    debug!("creation of indexes");
    // let mut rubber = Rubber::new(&settings.elasticsearch.connection_string)
    //     .with_nb_insert_threads(settings.elasticsearch.insert_thread_count);
    // rubber.initialize_templates()?;

    let settings = &settings;
    // FIXME We don't import admins from osm, just from cosmogony
    let pool =
        elasticsearch::remote::connection_pool_url(&settings.elasticsearch.connection_string)
            .await
            .map_err(|err| {
                format_err!(
                    "could not create elasticsearch connection pool: {}",
                    err.to_string()
                )
            })?;

    let client = pool
        .conn()
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    let list_documents = ListDocuments::new(Box::new(client.clone()));

    let parameters = ListDocumentsParameters {
        parameters: ListParameters {
            doc_type: String::from(Admin::doc_type()),
        },
    };
    let admin_stream = list_documents
        .execute(parameters)
        .await
        .map_err(|err| format_err!("could not retrieve admins: {}", err.to_string()))?;

    let admins = admin_stream
        .map(|v| serde_json::from_value(v).expect("cannot deserialize admin"))
        .collect::<Vec<Admin>>()
        .await;

    let admins_geofinder = admins.into_iter().collect::<AdminGeoFinder>();

    if settings
        .street
        .as_ref()
        .map(|street| street.import)
        .unwrap_or_else(|| false)
    {
        info!("Extracting streets from osm");
        let mut streets = streets(&mut osm_reader, &admins_geofinder, &settings)?;

        info!("computing street weight");
        compute_street_weight(&mut streets);

        let streets = futures::stream::iter(streets).map(StreetDoc::from);

        let config = IndexConfiguration {
            name: settings.dataset.clone(),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(include_str!("../../config/street/settings.json")),
            },
            mappings: IndexMappings {
                value: String::from(include_str!("../../config/street/mappings.json")),
            },
        };

        let config = serde_json::to_string(&config).map_err(|err| {
            format_err!(
                "could not serialize index configuration: {}",
                err.to_string()
            )
        })?;
        let generate_index = GenerateIndex::new(Box::new(client.clone()));
        let parameters = GenerateIndexParameters {
            config: Configuration { value: config },
            documents: Box::new(streets),
            doc_type: String::from(StreetDoc::DOC_TYPE),
            visibility: IndexVisibility::Public,
        };
        generate_index
            .execute(parameters)
            .await
            .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;
    }

    if settings
        .poi
        .as_ref()
        .map(|poi| poi.import)
        .unwrap_or_else(|| false)
    {
        let config = settings
            .poi
            .as_ref()
            .and_then(|poi| poi.config.clone())
            .unwrap_or_else(PoiConfig::default);

        // Ideally, this pois function would create a stream, which would then map and do other
        // stuff, and then be indexed
        //
        info!("Extracting pois from osm");
        let pois = pois(&mut osm_reader, &config, &admins_geofinder);

        let search_documents = SearchDocuments::new(Box::new(client.clone()));
        let pois: Vec<PoiDoc> = futures::stream::iter(pois)
            .then(|poi| compute_weight(poi))
            .then(|poi| add_address(poi, &search_documents))
            .map(PoiDoc::from)
            .collect()
            .await;

        let config = IndexConfiguration {
            name: settings.dataset.clone(),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(include_str!("../../config/poi/settings.json")),
            },
            mappings: IndexMappings {
                value: String::from(include_str!("../../config/poi/mappings.json")),
            },
        };

        let config = serde_json::to_string(&config).map_err(|err| {
            format_err!(
                "could not serialize index configuration: {}",
                err.to_string()
            )
        })?;
        let generate_index = GenerateIndex::new(Box::new(client));
        let parameters = GenerateIndexParameters {
            config: Configuration { value: config },
            documents: Box::new(futures::stream::iter(pois)),
            doc_type: String::from(PoiDoc::DOC_TYPE),
            visibility: IndexVisibility::Public,
        };
        generate_index
            .execute(parameters)
            .await
            .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;
    }
    Ok(())
}

// We need to allow for unused variables, because currently all the checks on
// args require the db-storage feature. If this feature is not used, then there
// is a warning
#[allow(unused_variables)]
fn validate_args(args: &Args) -> Result<(), mimirsbrunn::Error> {
    #[cfg(feature = "db-storage")]
    if args.db_file.is_some() {
        // If the user specified db_file, he must also specify db_buffer_size, or else!
        if args.db_buffer_size.is_none() {
            return Err(failure::format_err!("You need to specify database buffer size if you want to use database storage. Use --db-buffer-size"));
        }
    }
    #[cfg(feature = "db-storage")]
    if args.db_buffer_size.is_some() {
        // If the user specified db_buffer_size, he must also specify db_file, or else!
        if args.db_file.is_none() {
            return Err(failure::format_err!("You need to specify database file if you want to use database storage. Use --db-file"));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(Box::new(run)).await;
}

#[derive(Serialize)]
struct StreetDoc(Street);

impl StreetDoc {
    const DOC_TYPE: &'static str = "street";
}

impl From<Street> for StreetDoc {
    fn from(street: Street) -> Self {
        StreetDoc(street)
    }
}

impl Document for StreetDoc {
    fn doc_type(&self) -> &'static str {
        Self::DOC_TYPE
    }
    fn id(&self) -> String {
        self.0.id.clone()
    }
}

#[derive(Serialize)]
struct PoiDoc(Poi);

impl From<Poi> for PoiDoc {
    fn from(poi: Poi) -> Self {
        PoiDoc(poi)
    }
}

impl PoiDoc {
    const DOC_TYPE: &'static str = "poi";
}

impl Document for PoiDoc {
    fn doc_type(&self) -> &'static str {
        Self::DOC_TYPE
    }
    fn id(&self) -> String {
        self.0.id.clone()
    }
}
