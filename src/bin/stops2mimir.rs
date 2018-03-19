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
extern crate mimir;
extern crate mimirsbrunn;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
#[macro_use]
extern crate structopt;
extern crate failure;

use std::collections::HashMap;
use structopt::StructOpt;
use mimirsbrunn::stops::*;
use std::path::PathBuf;

const MAX_LAT: f64 = 90f64;
const MIN_LAT: f64 = -90f64;

const MAX_LON: f64 = 180f64;
const MIN_LON: f64 = -180f64;

#[derive(Debug, StructOpt)]
struct Args {
    /// NTFS stops.txt file.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: PathBuf,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string",
                default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Deprecated option.
    #[structopt(short = "C", long = "city-level")]
    city_level: Option<String>,
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
                coord: mimir::Coord::new(self.stop_lon, self.stop_lat),
                label: self.stop_name.clone(),
                weight: 0.,
                zip_codes: vec![],
                administrative_regions: vec![],
                name: self.stop_name,
                coverages: vec![],
                commercial_modes: vec![],
                physical_modes: vec![],
                timezone: format!(""),
                codes: vec![],
                properties: vec![],
                feed_publishers: vec![],
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

fn run(args: Args) -> Result<(), failure::Error> {
    let mut rdr = csv::Reader::from_path(&args.input).unwrap();
    let mut nb_stop_points = HashMap::new();
    let mut stops: Vec<mimir::Stop> = rdr.deserialize()
        .filter_map(|rc| rc.map_err(|e| warn!("skip csv line: {}", e)).ok())
        .filter_map(|stop: GtfsStop| {
            stop.incr_stop_point(&mut nb_stop_points);
            stop.try_into_with_warn()
        })
        .collect();
    set_weights(stops.iter_mut(), &nb_stop_points);
    import_stops(stops, &args.connection_string, &args.dataset)
}

fn main() {
    let _guard = mimir::logger_init();
    info!("Launching stops2mimir...");

    let args = Args::from_args();
    if args.city_level.is_some() {
        warn!("city-level option is deprecated, it now has no effect.");
    }
	if let Err(err) = run(args) {
        for cause in err.causes() {
            eprintln!("{}", cause);
        }
        std::process::exit(1);
    }
;
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
