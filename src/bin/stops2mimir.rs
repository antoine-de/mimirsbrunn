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

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate docopt;
extern crate csv;
extern crate mimir;
extern crate itertools;
extern crate mimirsbrunn;
#[macro_use]
extern crate log;

use std::rc::Rc;
use mimir::rubber::Rubber;
use docopt::Docopt;
use mimirsbrunn::utils::{format_label, get_zip_codes_from_admins};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use std::collections::HashMap;

const USAGE: &'static str = "
Usage:
    stops2mimir --help
    stops2mimir --input=<file> \
     [--connection-string=<connection-string>] [--dataset=<dataset>] [--city-level=<level>]

Options:
    -h, --help                Show this message.
    -i, --input=<file>        NTFS stops.txt file.
    -c, --connection-string=<connection-string>   \
                              Elasticsearch parameters [default: http://localhost:9200/munin].
    -d, --dataset=<dataset>   Name of the dataset [default: fr].
    -C, --city-level=<level>  City level to calculate weight [default: 8]
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_input: String,
    flag_dataset: String,
    flag_connection_string: String,
    flag_city_level: u32,
}

#[derive(Debug, Deserialize)]
struct Stop {
    stop_id: String,
    stop_lat: f64,
    stop_lon: f64,
    stop_name: String,
    location_type: Option<i32>,
    visible: Option<i32>,
    parent_station: Option<String>,
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
        "there is {}/{} stops without any admin",
        nb_unmatched,
        nb_matched + nb_unmatched
    );
}

// Update weight value for each stop_area from HashMap.
fn finalize_stop_area_weight<'a, It: Iterator<Item = &'a mut mimir::Stop>>(
    stops: It,
    nb_stop_points: &HashMap<String, u32>,
) {
    let max = *nb_stop_points.values().max().unwrap_or(&1) as f64;
    for stop in stops {
        if let Some(weight) = nb_stop_points.get(&stop.id) {
            stop.weight = *weight as f64 / max;
        }
    }
}

fn stop_to_mimir_stop(
    nb_stop_points: &mut HashMap<String, u32>,
    stop: Stop,
) -> Option<mimir::Stop> {
    match (stop.location_type, stop.parent_station) {
        (Some(0), Some(ref id)) |
        (None, Some(ref id)) if !id.is_empty() => {
            *nb_stop_points
                .entry(format!("stop_area:{}", id))
                .or_insert(0) += 1
        }
        _ => (),
    }
    if stop.location_type == Some(1) && stop.visible != Some(0) {
        Some(mimir::Stop {
            id: format!("stop_area:{}", stop.stop_id), // prefix to match navitia's id
            coord: mimir::Coord::new(stop.stop_lat, stop.stop_lon),
            label: stop.stop_name.clone(),
            weight: 0.,
            zip_codes: vec![],
            administrative_regions: vec![],
            name: stop.stop_name,
        })
    } else {
        None
    }
}

fn main() {
    mimir::logger_init().unwrap();
    info!("Launching stops2mimir...");

    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.deserialize())
        .unwrap_or_else(|e| e.exit());

    info!("creation of indexes");
    let mut rubber = Rubber::new(&args.flag_connection_string);
    let mut rdr = csv::Reader::from_path(args.flag_input).unwrap();

    let mut nb_stop_points = HashMap::new();

    let mut stops: Vec<mimir::Stop> = rdr.deserialize()
        .filter_map(|rc| {
            rc.map_err(|e| warn!("skip csv line because: {}", e)).ok()
        })
        .filter_map(|stop: Stop| stop_to_mimir_stop(&mut nb_stop_points, stop))
        .collect();

    attach_stops_to_admins(stops.iter_mut(), &mut rubber, args.flag_city_level);

    finalize_stop_area_weight(stops.iter_mut(), &nb_stop_points);

    info!("Importing {} stops into Mimir", stops.len());
    let nb_stops = rubber.index(&args.flag_dataset, stops.iter()).unwrap();

    info!("Nb of indexed stops: {}", nb_stops);

}

#[test]
fn test_load_stops() {
    use itertools::Itertools;
    let mut rdr = csv::Reader::from_path("./tests/fixtures/stops.txt".to_string()).unwrap();

    let mut nb_stop_points = HashMap::new();
    let stops: Vec<mimir::Stop> = rdr.deserialize()
        .filter_map(Result::ok)
        .filter_map(|stop| stop_to_mimir_stop(&mut nb_stop_points, stop))
        .collect();
    let ids: Vec<_> = stops.iter().map(|s| s.id.clone()).sorted();
    assert_eq!(
        ids,
        vec![
            "stop_area:SA:main_station",
            "stop_area:SA:second_station",
            "stop_area:SA:station_no_city",
            "stop_area:SA:weight_1_station",
            "stop_area:SA:weight_3_station",
        ]
    );
    let weights: Vec<_> = ids.iter().map(|id| nb_stop_points.get(id)).collect();
    assert_eq!(weights, vec![Some(&1), Some(&1), None, Some(&1), Some(&3)]);
}
