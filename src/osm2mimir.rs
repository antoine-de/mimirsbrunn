// Copyright © 2023, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Hove (www.kisio.com).
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

use futures::stream::StreamExt;
use mimir::domain::model::configuration::ContainerConfig;
use snafu::{ResultExt, Snafu};
use tracing::instrument;

use crate::{
    admin_geofinder::AdminGeoFinder,
    osm_reader::street::streets,
    settings::{admin_settings::AdminSettings, osm2mimir as settings},
    utils::template::update_templates,
};
use mimir::{
    adapters::secondary::elasticsearch::{self, ElasticsearchStorage},
    domain::ports::{primary::generate_index::GenerateIndex, secondary::remote::Remote},
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("OSM PBF Reader Error: {}", source))]
    OsmPbfReader { source: crate::osm_reader::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Street Extraction from OSM PBF Error {}", source))]
    StreetOsmExtraction {
        source: crate::osm_reader::street::Error,
    },

    #[snafu(display("Street Index Creation Error {}", source))]
    StreetIndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Poi Extraction from OSM PBF Error {}", source))]
    PoiOsmExtraction {
        source: crate::osm_reader::poi::Error,
    },

    #[snafu(display("Poi Index Creation Error {}", source))]
    PoiIndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Elasticsearch Configuration {}", source))]
    StreetElasticsearchConfiguration { source: common::config::Error },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },

    #[snafu(display("Admin Retrieval Error {}", details))]
    AdminRetrieval { details: String },
}

#[instrument(skip_all)]
async fn import_streets(
    streets: Vec<places::street::Street>,
    client: &ElasticsearchStorage,
    config: &ContainerConfig,
) -> Result<(), Error> {
    let streets = streets
        .into_iter()
        .map(|street| street.set_weight_from_admins());

    let _index = client
        .generate_index(config, futures::stream::iter(streets))
        .await
        .context(StreetIndexCreationSnafu)?;

    Ok(())
}

#[instrument(skip_all)]
async fn import_pois(
    osm_reader: &mut crate::osm_reader::OsmPbfReader,
    admins_geofinder: &AdminGeoFinder,
    poi_config: &crate::osm_reader::poi::PoiConfig,
    client: &ElasticsearchStorage,
    config: &ContainerConfig,
    max_distance_reverse: usize,
) -> Result<(), Error> {
    // This function rely on AdminGeoFinder::get_objs_and_deps
    // which use all available cpu/cores to decode osm file and cannot be limited by tokio runtime
    let pois = crate::osm_reader::poi::pois(osm_reader, poi_config, admins_geofinder)
        .context(PoiOsmExtractionSnafu)?;

    let pois: Vec<places::poi::Poi> = futures::stream::iter(pois)
        .map(crate::osm_reader::poi::compute_weight)
        .then(|poi| crate::osm_reader::poi::add_address(client, poi, max_distance_reverse))
        .collect()
        .await;

    let _ = client
        .generate_index(config, futures::stream::iter(pois))
        .await
        .context(PoiIndexCreationSnafu)?;

    Ok(())
}

pub async fn run(
    opts: settings::Opts,
    settings: settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut osm_reader =
        crate::osm_reader::make_osm_reader(&opts.input).context(OsmPbfReaderSnafu)?;

    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch.clone())
        .await
        .context(ElasticsearchConnectionSnafu)?;

    // Update all the template components and indexes
    if settings.update_templates {
        update_templates(&client, opts.config_dir).await?;
    }

    let admin_settings = AdminSettings::build(&settings.admins);

    let admins_geofinder = AdminGeoFinder::build(&admin_settings, &client).await?;

    if settings.streets.import {
        let streets = streets(
            &mut osm_reader,
            &admins_geofinder,
            &settings.streets.exclusions,
            #[cfg(feature = "db-storage")]
            settings.database.as_ref(),
        )
        .context(StreetOsmExtractionSnafu)?;

        import_streets(streets, &client, &settings.container_street).await?;
    }

    if settings.pois.import {
        import_pois(
            &mut osm_reader,
            &admins_geofinder,
            &settings.pois.config.clone().unwrap_or_default(),
            &client,
            &settings.container_poi,
            settings.pois.max_distance_reverse,
        )
        .await?;
    }

    Ok(())
}
