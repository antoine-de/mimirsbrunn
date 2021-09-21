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
