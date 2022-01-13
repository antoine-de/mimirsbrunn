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

/// In this module we put the code related to stops, that need to draw on 'places', 'mimir',
/// 'common', and 'config' (ie all the workspaces that make up mimirsbrunn).
use futures::stream::{Stream, TryStreamExt};
use mimir::domain::model::configuration::{ContainerConfig, PhysicalModeWeight};
use snafu::{ResultExt, Snafu};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::BuildHasherDefault;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

use crate::admin_geofinder::AdminGeoFinder;
use crate::labels;
use mimir::adapters::secondary::elasticsearch::{self, ElasticsearchStorage};
use mimir::domain::ports::primary::{generate_index::GenerateIndex, list_documents::ListDocuments};
use places::admin::Admin;
use places::stop::Stop;

#[derive(Debug, Snafu)]
pub enum Error {
    // #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    // Settings { source: settings::Error },
    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchPool {
        source: elasticsearch::remote::Error,
    },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    // #[snafu(display("Cosmogony Error: {}", details))]
    // Cosmogony { details: String },
    #[snafu(display("Index Generation Error {}", source))]
    IndexGeneration {
        source: mimir::domain::model::error::Error,
    },

    // transit_model uses failure::Error, which does not implement std::Error, so
    // we use a String to get the error message instead.
    #[snafu(display("Transit Model Error {}", details))]
    TransitModel { details: String },

    #[snafu(display("Admin Retrieval Error {}", details))]
    AdminRetrieval { details: String },
}

pub fn initialize_weights<'a, It, S: ::std::hash::BuildHasher>(
    stops: It,
    nb_stop_points: &HashMap<String, u32, S>,
) where
    It: Iterator<Item = &'a mut Stop>,
{
    let max = *nb_stop_points.values().max().unwrap_or(&1) as f64;
    for stop in stops {
        stop.weight = if let Some(weight) = nb_stop_points.get(&stop.id) {
            *weight as f64 / max
        } else {
            0.0
        };
    }
}

pub fn make_weight(stop: &mut Stop, physical_mode_weight: &Option<Vec<PhysicalModeWeight>>) {
    // Admin weight
    let mut admin_weight = stop
        .administrative_regions
        .iter()
        .filter(|adm| adm.is_city())
        .map(|adm| adm.weight)
        .next()
        .unwrap_or(0.0);
    // FIXME: 1024, automagic!
    // It's a factor used to bring the stop weight and the admin weight in the same order of
    // magnitude...
    // We then use a log to compress the distance between low admin weight and high ones.
    admin_weight = admin_weight * 1024.0 + 1.0;
    admin_weight = admin_weight.log10();

    let mut result = Vec::new();
    if let Some(ph_weight) = physical_mode_weight {
        result = stop
            .physical_modes
            .iter()
            .map(|mode| {
                let pm_w = ph_weight.iter().find(|&md| md.id == mode.id);
                match pm_w {
                    Some(value) => value.weight as f64,
                    _ => {
                        warn!(
                            "Physical mode, id: {} name: {}, not found in mimir config.",
                            mode.id, mode.name
                        );
                        0.0
                    }
                }
            })
            .filter(|weight| !weight.is_nan())
            .collect();
    }

    result.push(stop.weight);
    result.push(admin_weight);
    let sum: f64 = Iterator::sum(result.iter());
    stop.weight = sum / (result.len() as f64);
}

fn attach_stop(stop: &mut Stop, admins: Vec<Arc<Admin>>) {
    let admins_iter = admins.iter().map(|a| a.deref());
    let country_codes = places::admin::find_country_codes(admins_iter.clone());

    stop.label = labels::format_stop_label(&stop.name, admins_iter, &country_codes);
    stop.zip_codes = places::admin::get_zip_codes_from_admins(&admins);

    stop.country_codes = country_codes;
    stop.administrative_regions = admins;
}

/// Attach the stops to administrative regions
///
/// The admins are loaded from Elasticsearch and stored in a quadtree
/// We attach a stop with all the admins that have a boundary containing
/// the coordinate of the stop
async fn attach_stops_to_admins<'a, It: Iterator<Item = &'a mut Stop>>(
    stops: It,
    client: &ElasticsearchStorage,
) -> Result<(), Error> {
    match client.list_documents().await {
        Ok(stream) => {
            let admins: Vec<Admin> = stream.try_collect().await.context(IndexGenerationSnafu)?;

            if admins.is_empty() {
                return Err(Error::AdminRetrieval {
                    details: String::from("no admin retrieved to enrich stops"),
                });
            }
            info!("{} admins retrieved from ES ", admins.len());
            let admins_geofinder = admins.into_iter().collect::<AdminGeoFinder>();

            let mut nb_unmatched = 0u32;
            let mut nb_matched = 0u32;
            // FIXME Opportunity for concurrent work
            for mut stop in stops {
                let admins = admins_geofinder.get(&stop.coord);

                if admins.is_empty() {
                    nb_unmatched += 1;
                } else {
                    nb_matched += 1;
                }

                attach_stop(&mut stop, admins);
            }

            info!(
                "there are {}/{} stops without any admin",
                nb_unmatched,
                nb_matched + nb_unmatched
            );
            Ok(())
        }
        Err(_) => Err(Error::AdminRetrieval {
            details: String::from("Could not retrieve admins to enrich stops"),
        }),
    }
}

/// Stores the stops found in the 'input' directory, in Elasticsearch, with the given
/// configuration.
///
/// The main part of this function is to actually create a list of stops
/// from the information found in the NTFS directory.
pub async fn index_ntfs(
    input: PathBuf,
    config: &ContainerConfig,
    physical_mode_weight: &Option<Vec<PhysicalModeWeight>>,
    client: &ElasticsearchStorage,
) -> Result<(), Error> {
    let navitia = transit_model::ntfs::read(&input).map_err(|err| Error::TransitModel {
        details: format!(
            "Could not read transit model from {}: {}",
            input.display(),
            err.to_string()
        ),
    })?;
    info!("Build number of stops per stoparea");
    let nb_stop_points: HashMap<String, u32, BuildHasherDefault<DefaultHasher>> = navitia
        .stop_areas
        .iter()
        .map(|(idx, sa)| {
            let id = places::utils::normalize_id("stop_area", &sa.id);
            let nb_stop_points = navitia
                .get_corresponding_from_idx::<_, transit_model::objects::StopPoint>(idx)
                .len();
            (id, nb_stop_points as u32)
        })
        .collect();

    info!("Make mimir stops from navitia stops");
    let mut stops: Vec<Stop> = navitia
        .stop_areas
        .iter()
        .map(|(idx, sa)| places::stop::to_mimir(idx, sa, &navitia))
        .collect();

    info!("Initialize stops weights");
    initialize_weights(stops.iter_mut(), &nb_stop_points);

    info!("Attach stops to admins");
    attach_stops_to_admins(stops.iter_mut(), client).await?;

    // FIXME Should be done concurrently (for_each_concurrent....)
    info!("Build stops weight by physical modes and city population");
    for stop in &mut stops {
        stop.coverages.push(config.dataset.clone());
        make_weight(stop, &physical_mode_weight);
    }
    tracing::info!("Beginning to import stops into elasticsearch.");
    import_stops(client, config, futures::stream::iter(stops)).await
}

// FIXME Should not be ElasticsearchStorage, but rather a trait GenerateIndex
pub async fn import_stops<S>(
    client: &ElasticsearchStorage,
    config: &ContainerConfig,
    stops: S,
) -> Result<(), Error>
where
    S: Stream<Item = Stop> + Send + Sync + Unpin + 'static,
{
    client
        .generate_index(config, stops)
        .await
        .context(IndexGenerationSnafu)?;

    Ok(())
}
