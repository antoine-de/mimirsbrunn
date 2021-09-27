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
use slog_scope::{info, warn};

use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::{
    model::index::IndexVisibility, ports::primary::generate_index::GenerateIndex,
};
use mimir2::{
    adapters::secondary::elasticsearch::{self, ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ},
    domain::ports::secondary::remote::Remote,
};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::osm_reader::make_osm_reader;
use mimirsbrunn::osm_reader::poi::{add_address, compute_weight, pois, PoiConfig};
use mimirsbrunn::osm_reader::street::{compute_street_weight, streets};
use mimirsbrunn::settings::osm2mimir::{Args, Settings};
use places::poi::Poi;

const POI_REVERSE_GEOCODING_CONCURRENCY: usize = 8;

async fn run(args: Args) -> Result<(), mimirsbrunn::Error> {
    let input = args.input.clone(); // we save the input, because args will be consumed by settings.
    validate_args(&args)?;
    let settings = &Settings::new(args.clone())?;

    let mut osm_reader = make_osm_reader(&input)?;

    let import_streets_enabled = settings
        .street
        .as_ref()
        .map(|street| street.import)
        .unwrap_or_else(|| false);

    let import_poi_enabled = settings
        .poi
        .as_ref()
        .map(|poi| poi.import)
        .unwrap_or_else(|| false);

    if !import_streets_enabled && !import_poi_enabled {
        return Err(format_err!(
            "Neither streets nor POIs import is enabled. Nothing to do.\n\
             Use --import-way=true or --import-poi=true"
        ));
    }

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
        .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    let admins_geofinder: AdminGeoFinder = match client.list_documents().await {
        Ok(stream) => {
            stream
                .map(|admin| admin.expect("could not parse admin"))
                .collect()
                .await
        }
        Err(err) => {
            warn!("administratives regions not found in es db. {:?}", err);
            std::iter::empty().collect()
        }
    };

    if import_streets_enabled {
        info!("Extracting streets from osm");
        let mut streets = streets(&mut osm_reader, &admins_geofinder, settings)?;

        info!("computing street weight");
        compute_street_weight(&mut streets);

        let streets = futures::stream::iter(streets);
        let street_config = args.get_street_config()?;

        client
            .generate_index(street_config, streets, IndexVisibility::Public)
            .await
            .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;
    }

    if import_poi_enabled {
        let config = settings
            .poi
            .as_ref()
            .and_then(|poi| poi.config.clone())
            .unwrap_or_else(PoiConfig::default);

        // Ideally, this pois function would create a stream, which would then map and do other
        // stuff, and then be indexed
        info!("Extracting pois from osm");
        let pois = pois(&mut osm_reader, &config, &admins_geofinder);

        let pois: Vec<Poi> = futures::stream::iter(pois)
            .map(compute_weight)
            .map(|poi| add_address(&client, poi))
            .buffer_unordered(POI_REVERSE_GEOCODING_CONCURRENCY)
            .collect()
            .await;

        let poi_config = args.get_poi_config()?;

        client
            .generate_index(
                poi_config,
                futures::stream::iter(pois),
                IndexVisibility::Public,
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elasticsearch::ElasticsearchStorage;
    use common::config::load_es_config_for;
    use common::document::ContainerDocument;
    use futures::TryStreamExt;
    use mimir2::domain::model::query::Query;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::domain::ports::primary::search_documents::SearchDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use mimirsbrunn::admin::index_cosmogony;
    use places::{admin::Admin, street::Street, Place};
    use structopt::StructOpt;

    fn elasticsearch_test_url() -> String {
        std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
    }

    async fn index_cosmogony_admins(client: &ElasticsearchStorage) {
        index_cosmogony(
            "./tests/fixtures/cosmogony.json".into(),
            vec![],
            load_es_config_for::<Admin>(
                None,
                None,
                vec!["container.dataset=osm2mimir-test".into()],
            )
            .unwrap(),
            client,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn should_correctly_index_osm_streets_and_pois() {
        let _guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");

        index_cosmogony_admins(&client).await;

        let storage_args = if cfg!(feature = "db-storage") {
            vec!["--db-file=test-db.sqlite3", "--db-buffer-size=10"]
        } else {
            vec![]
        };

        let args = Args::from_iter(
            [
                "osm2mimir",
                "--input=./tests/fixtures/osm_fixture.osm.pbf",
                "--dataset=osm2mimir-test",
                "--import-way=true",
                "--import-poi=true",
                &format!("-c={}", elasticsearch_test_url()),
            ]
            .iter()
            .copied()
            .chain(storage_args),
        );

        let _res = mimirsbrunn::utils::launch_async_args(run, args).await;

        let search = |query: &str| {
            let client = client.clone();
            let query: String = query.into();
            async move {
                client
                    .search_documents(
                        vec![
                            Street::static_doc_type().into(),
                            Poi::static_doc_type().into(),
                        ],
                        Query::QueryString(format!("full_label.prefix:({})", query)),
                    )
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|json| serde_json::from_value::<Place>(json).unwrap())
                    .collect::<Vec<Place>>()
            }
        };

        let streets: Vec<Street> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        assert_eq!(streets.len(), 13);

        // Basic street search
        let results = search("Rue des Près").await;
        assert_eq!(results[0].label(), "Rue des Près (Livry-sur-Seine)");
        assert_eq!(
            results
                .iter()
                .filter(
                    |place| place.is_street() && place.label() == "Rue des Près (Livry-sur-Seine)"
                )
                .count(),
            1,
            "Only 1 'Rue des Près' is expected"
        );

        // All ways with same name in the same city are merged into a single street
        let results = search("Rue du Four à Chaux").await;
        assert_eq!(
            results.iter()
                .filter(|place| place.label() == "Rue du Four à Chaux (Livry-sur-Seine)")
                .count(),
            1,
            "Only 1 'Rue du Four à Chaux' is expected as all ways the same name should be merged into 1 street."
        );
        assert_eq!(
            results[0].id(),
            "street:osm:way:40812939",
            "The way with minimum way_id should be used as street id."
        );

        // Street admin is based on a middle node.
        // (Here the first node is located outside Melun)
        let results = search("Rue Marcel Houdet").await;
        assert_eq!(results[0].label(), "Rue Marcel Houdet (Melun)");
        assert!(results[0]
            .admins()
            .iter()
            .filter(|a| a.is_city())
            .any(|a| a.name == "Melun"));

        // Basic search for Poi by label
        let res = search("Le-Mée-sur-Seine Courtilleraies").await;
        assert_eq!(
            res[0].poi().expect("Place should be a poi").poi_type.id,
            "poi_type:amenity:post_office"
        );

        // highway=bus_stop should not be indexed
        let res = search("Grand Châtelet").await;
        assert!(
            res.is_empty(),
            "'Grand Châtelet' (highway=bus_stop) should not be found."
        );

        // "Rue de Villiers" is at the exact neighborhood between two cities, a
        // document must be added for both.
        let results = search("Rue de Villiers").await;
        assert!(["Neuilly-sur-Seine", "Levallois-Perret"]
            .iter()
            .all(|city| {
                results.iter().any(|poi| {
                    poi.admins()
                        .iter()
                        .filter(|a| a.is_city())
                        .any(|admin| &admin.name == city)
                })
            }));
    }
}
