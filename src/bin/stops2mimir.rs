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

extern crate csv;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate mimir;
extern crate mimirsbrunn;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::rc::Rc;
use mimir::rubber::Rubber;
use mimirsbrunn::utils::{format_label, get_zip_codes_from_admins};
use mimir::MimirObject;
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use std::collections::HashMap;
use structopt::StructOpt;

const GLOBAL_STOP_INDEX_NAME: &'static str = "munin_global_stops";
const MAX_LAT: f64 = 90f64;
const MIN_LAT: f64 = -90f64;

const MAX_LON: f64 = 180f64;
const MIN_LON: f64 = -180f64;

#[derive(Debug, StructOpt)]
struct Args {
    /// NTFS stops.txt file.
    #[structopt(short = "i", long = "input")]
    input: String,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string",
                default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// City level to calculate weight.
    #[structopt(short = "C", long = "city-level", default_value = "8")]
    city_level: u32,
}

#[derive(Deserialize, Debug)]
enum StopConversionErr {
    ///StopArea is invisible in Autocomplete
    InvisibleStop,
    ///The stop in the line is not a StopArea
    NotStopArea,
    ///Values of one or more attributes are not valid
    InvalidStop(String),
}

#[derive(Debug, Deserialize)]
struct GtfsStop {
    stop_id: String,
    stop_lat: f64,
    stop_lon: f64,
    stop_name: String,
    location_type: Option<i32>,
    visible: Option<i32>,
    parent_station: Option<String>,
}

impl GtfsStop {
    fn incr_stop_point(&self, nb_stop_points: &mut HashMap<String, u32>) {
        match (self.location_type, &self.parent_station) {
            (Some(0), &Some(ref id)) | (None, &Some(ref id)) if !id.is_empty() => {
                *nb_stop_points
                    .entry(format!("stop_area:{}", id))
                    .or_insert(0) += 1
            }
            _ => (),
        }
    }
    // to be moved when TryInto is stablilized
    fn try_into(self) -> Result<mimir::Stop, StopConversionErr> {
        if self.location_type != Some(1) {
            Err(StopConversionErr::NotStopArea)
        } else if self.visible == Some(0) {
            Err(StopConversionErr::InvisibleStop)
        } else if self.stop_lat <= MIN_LAT || self.stop_lat >= MAX_LAT || self.stop_lon <= MIN_LON
            || self.stop_lon >= MAX_LON
        {
            //Here we return an error message
            Err(StopConversionErr::InvalidStop(format!(
                "Invalid lon {:?} or lat {:?} for stop {:?}",
                self.stop_lon, self.stop_lat, self.stop_name
            )))
        } else {
            Ok(mimir::Stop {
                id: format!("stop_area:{}", self.stop_id), // prefix to match navitia's id
                coord: mimir::Coord::new(self.stop_lat, self.stop_lon),
                label: self.stop_name.clone(),
                weight: 0.,
                zip_codes: vec![],
                administrative_regions: vec![],
                name: self.stop_name,
                coverages: vec![],
            })
        }
    }
    fn try_into_with_warn(self) -> Option<mimir::Stop> {
        match self.try_into() {
            Ok(s) => Some(s),
            Err(StopConversionErr::InvisibleStop) => None,
            Err(StopConversionErr::NotStopArea) => None,
            Err(StopConversionErr::InvalidStop(msg)) => {
                warn!("skip csv line: {}", msg);
                None
            }
        }
    }
}

fn attach_stop(stop: &mut mimir::Stop, admins: Vec<Rc<mimir::Admin>>, city_level: u32) {
    stop.administrative_regions = admins;
    stop.label = format_label(&stop.administrative_regions, city_level, &stop.name);
    stop.zip_codes = get_zip_codes_from_admins(&stop.administrative_regions);
}

/// Attach the stops to administrative regions
///
/// The admins are loaded from Elasticsearch and stored in a quadtree
/// We attach a stop with all the admins that have a boundary containing
/// the coordinate of the stop
fn attach_stops_to_admins<'a, It: Iterator<Item = &'a mut mimir::Stop>>(
    stops: It,
    rubber: &mut Rubber,
    city_level: u32,
) {
    let admins = rubber.get_all_admins().unwrap_or_else(|_| {
        info!("Administratives regions not found in elasticsearch db");
        vec![]
    });

    info!("{} administrative regions loaded from mimir", admins.len());

    let admins_geofinder = admins.into_iter().collect::<AdminGeoFinder>();

    let mut nb_unmatched = 0u32;
    let mut nb_matched = 0u32;
    for mut stop in stops {
        let admins = admins_geofinder.get(&stop.coord);

        if admins.is_empty() {
            nb_unmatched += 1;
        } else {
            nb_matched += 1;
        }

        attach_stop(&mut stop, admins, city_level);
    }

    info!(
        "there are {}/{} stops without any admin",
        nb_unmatched,
        nb_matched + nb_unmatched
    );
}

// Update weight value for each stop_area from HashMap and stop's coverage
fn finalize_stop_area<'a, It: Iterator<Item = &'a mut mimir::Stop>>(
    stops: It,
    nb_stop_points: &HashMap<String, u32>,
    dataset: &str,
) {
    let max = *nb_stop_points.values().max().unwrap_or(&1) as f64;
    for stop in stops {
        if let Some(weight) = nb_stop_points.get(&stop.id) {
            stop.weight = *weight as f64 / max;
        }
        stop.coverages.push(dataset.to_string());
    }
}

/// merge the stops from all the different indexes
/// for the moment the merge is very simple and uses only the ID
/// (and we take the data from the first stop inserted)
fn merge_stops<It: IntoIterator<Item = mimir::Stop>>(
    stops: It,
) -> Box<Iterator<Item = mimir::Stop>> {
    let mut stops_by_id = HashMap::<String, mimir::Stop>::new();
    for mut stop in stops.into_iter() {
        let cov = std::mem::replace(&mut stop.coverages, vec![]);
        stops_by_id
            .entry(stop.id.clone())
            .or_insert(stop)
            .coverages
            .extend(cov.into_iter());
    }
    Box::new(stops_by_id.into_iter().map(|(_, v)| v))
}

fn get_all_stops(rubber: &mut Rubber, index: String) -> Result<Vec<mimir::Stop>, String> {
    rubber
        .get_all_objects_from_index(&index)
        .map_err(|e| e.to_string())
}

fn update_global_stop_index<'a, It: Iterator<Item = &'a mimir::Stop>>(
    rubber: &mut Rubber,
    stops: It,
    dataset: &str,
) -> Result<String, String> {
    let dataset_index =
        mimir::rubber::get_main_type_and_dataset_index(mimir::Stop::doc_type(), dataset);
    let stops_indexes = rubber
        .get_all_aliased_index(&mimir::rubber::get_main_type_index(mimir::Stop::doc_type()))?
        .into_iter()
        .filter(|&(_, ref aliases)| !aliases.contains(&dataset_index))
        .map(|(index, _)| index);

    let all_other_es_stops: Vec<_> = stops_indexes
        .map(|index| get_all_stops(rubber, index).unwrap())
        .flat_map(|stops| stops.into_iter())
        .collect();

    let all_es_stops = all_other_es_stops
        .into_iter()
        .chain(stops.into_iter().cloned());

    let all_merged_stops = merge_stops(all_es_stops);
    let es_index_name = mimir::rubber::get_date_index_name(GLOBAL_STOP_INDEX_NAME);

    rubber.create_index(&es_index_name)?;

    let nb_stops_added = rubber
        .bulk_index(&es_index_name, all_merged_stops)
        .map_err(|e| e.to_string())?;
    info!("{} stops added in the global index", nb_stops_added);
    // create global index
    // fill structure for each stop indexes
    Ok(es_index_name)
}

// publish the global stop index
// alias the new index to the global stop alias, and remove the old index
fn publish_global_index(rubber: &mut Rubber, new_global_index: &str) -> Result<(), String> {
    let last_global_indexes: Vec<_> = rubber
        .get_all_aliased_index(GLOBAL_STOP_INDEX_NAME)?
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k != new_global_index)
        .collect();
    rubber.alias(
        GLOBAL_STOP_INDEX_NAME,
        &vec![new_global_index.to_string()],
        &last_global_indexes,
    )?;

    for index in last_global_indexes {
        rubber.delete_index(&index)?;
    }
    Ok(())
}

fn main() {
    mimir::logger_init().unwrap();
    info!("Launching stops2mimir...");

    let args = Args::from_args();

    info!("creation of indexes");
    let mut rubber = Rubber::new(&args.connection_string);
    let mut rdr = csv::Reader::from_path(args.input).unwrap();

    let mut nb_stop_points = HashMap::new();

    let mut stops: Vec<mimir::Stop> = rdr.deserialize()
        .filter_map(|rc| rc.map_err(|e| warn!("skip csv line: {}", e)).ok())
        .filter_map(|stop: GtfsStop| {
            stop.incr_stop_point(&mut nb_stop_points);
            stop.try_into_with_warn()
        })
        .collect();

    attach_stops_to_admins(stops.iter_mut(), &mut rubber, args.city_level);

    finalize_stop_area(stops.iter_mut(), &nb_stop_points, &args.dataset);

    let global_index = update_global_stop_index(&mut rubber, stops.iter(), &args.dataset).unwrap();

    info!("Importing {} stops into Mimir", stops.len());
    let nb_stops = rubber.index(&args.dataset, stops.iter()).unwrap();
    info!("Nb of indexed stops: {}", nb_stops);

    publish_global_index(&mut rubber, &global_index).unwrap();
}

#[test]
fn test_load_stops() {
    use itertools::Itertools;
    let mut rdr = csv::Reader::from_path("./tests/fixtures/stops.txt".to_string()).unwrap();

    let mut nb_stop_points = HashMap::new();
    let stops: Vec<mimir::Stop> = rdr.deserialize()
        .filter_map(Result::ok)
        .filter_map(|stop: GtfsStop| {
            stop.incr_stop_point(&mut nb_stop_points);
            stop.try_into_with_warn()
        })
        .collect();
    let ids: Vec<_> = stops.iter().map(|s| s.id.clone()).sorted();
    assert_eq!(
        ids,
        vec![
            "stop_area:SA:known_by_all_dataset",
            "stop_area:SA:main_station",
            "stop_area:SA:second_station",
            "stop_area:SA:station_no_city",
            "stop_area:SA:weight_1_station",
            "stop_area:SA:weight_3_station",
        ]
    );
    let weights: Vec<_> = ids.iter().map(|id| nb_stop_points.get(id)).collect();
    assert_eq!(
        weights,
        vec![None, Some(&1), Some(&1), None, Some(&1), Some(&3)]
    );
}
