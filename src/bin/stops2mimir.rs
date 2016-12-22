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

extern crate rustc_serialize;
extern crate docopt;
extern crate csv;
extern crate mimir;
use mimir::rubber::Rubber;
#[macro_use]
extern crate log;

use docopt::Docopt;

const USAGE: &'static str =
    "
Usage:
    stops2mimir --help
    stops2mimir --input=<file> \
     [--connection-string=<connection-string>] [--dataset=<dataset>]

Options:
    -h, --help               Show this message.
    -i, --input=<file>       NTFS stops.txt file.
    -c, --connection-string=<connection-string>   \
                             Elasticsearch parameters [default: http://localhost:9200/munin].
    -d, --dataset=<dataset>  Name of the dataset [default: fr].
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_input: String,
    flag_dataset: String,
    flag_connection_string: String,
}

struct StopPointIter<'a, R: std::io::Read + 'a> {
    iter: csv::StringRecords<'a, R>,
    stop_id_pos: usize,
    stop_lat_pos: usize,
    stop_lon_pos: usize,
    stop_name_pos: usize,
    location_type_pos: Option<usize>,
    stop_visible_pos: Option<usize>,
}

impl<'a, R: std::io::Read + 'a> StopPointIter<'a, R> {
    fn new(r: &'a mut csv::Reader<R>) -> csv::Result<Self> {
        let headers = try!(r.headers());
        let get_optional_pos = |name| headers.iter().position(|s| s == name);

        let get_pos = |field| {
            get_optional_pos(field)
                .ok_or(csv::Error::Decode(format!("Invalid file, cannot find column '{}'", field)))
        };

        Ok(StopPointIter {
            iter: r.records(),
            stop_id_pos: try!(get_pos("stop_id")),
            stop_lat_pos: try!(get_pos("stop_lat")),
            stop_lon_pos: try!(get_pos("stop_lon")),
            stop_name_pos: try!(get_pos("stop_name")),
            location_type_pos: get_optional_pos("location_type"),
            stop_visible_pos: get_optional_pos("visible"),
        })
    }
    fn get_location_type(&self, record: &[String]) -> Option<u8> {
        self.location_type_pos.and_then(|pos| record.get(pos).and_then(|s| s.parse().ok()))
    }
    fn get_visible(&self, record: &[String]) -> Option<u8> {
        self.stop_visible_pos.and_then(|pos| record.get(pos).and_then(|s| s.parse().ok()))
    }
}

impl<'a, R: std::io::Read + 'a> Iterator for StopPointIter<'a, R> {
    type Item = csv::Result<mimir::Stop>;
    fn next(&mut self) -> Option<Self::Item> {
        fn get(record: &[String], pos: usize) -> csv::Result<&str> {
            record.get(pos)
                .map(|s| s.as_str())
                .ok_or_else(|| csv::Error::Decode(format!("Failed accessing record '{}'.", pos)))
        }
        fn parse_f64(s: &str) -> csv::Result<f64> {
            s.parse()
                .map_err(|_| csv::Error::Decode(format!("Failed converting '{}' from str.", s)))
        }

        fn is_valid_stop_area(location_type: &Option<u8>, visible: &Option<u8>) -> csv::Result<()> {
            if *location_type == Some(1) && *visible != Some(0) {
                Ok(())
            } else {
                Err(csv::Error::Decode("Not stop_area.".to_string()))
            }
        }
        self.iter.next().map(|r| {
            r.and_then(|r| {
                let location_type = self.get_location_type(&r);
                let visible = self.get_visible(&r);
                try!(is_valid_stop_area(&location_type, &visible));

                let stop_id = try!(get(&r, self.stop_id_pos));
                let stop_lat = try!(get(&r, self.stop_lat_pos));
                let stop_lat = try!(parse_f64(stop_lat));
                let stop_lon = try!(get(&r, self.stop_lon_pos));
                let stop_lon = try!(parse_f64(stop_lon));
                let stop_name = try!(get(&r, self.stop_name_pos));
                Ok(mimir::Stop {
                    id: stop_id.to_string(),
                    coord: mimir::Coord::new(stop_lat, stop_lon),
                    label: stop_name.to_string(),
                    weight: 1,
                    zip_codes: vec![],
                    administrative_regions: vec![],
                    name: stop_name.to_string(),
                })
            })
        })
    }
}

fn main() {
    info!("Launching stops2mimir...");

    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.decode())
        .unwrap_or_else(|e| e.exit());

    info!("creation of indexes");
    let mut rubber = Rubber::new(&args.flag_connection_string);
    let mut rdr = csv::Reader::from_file(args.flag_input)
        .unwrap()
        .double_quote(true);

    let stops: Vec<mimir::Stop> = StopPointIter::new(&mut rdr)
        .unwrap()
        .filter_map(|rc| {
            rc.map_err(|e| error!("error at csv line decoding : {}", e))
                .ok()
        })
        .collect();

    info!("Importing stops into Mimir");
    let nb_stops = rubber.index("stops", &args.flag_dataset, stops.iter())
        .unwrap();

    info!("Nb of indexed stops: {}", nb_stops);

}

#[test]
fn test_load_stops() {
    // stops.txt:
    // BGT:SP:gpualsa3 : StopPoint object
    // BGT:SA:gpualsa3: StopArea valid
    // BGT:SA:bou14ju: StopArea valid with visible is empty
    // OLS:SA:OCTOB: invisible StopArea
    // OLS:SA:daudet: StopArea object without X coord
    // BGT:SA:boualou2: StopArea object without X coord
    //
    let mut rdr = csv::Reader::from_file("./tests/fixtures/stops.txt".to_string())
        .unwrap()
        .double_quote(true);

    let stops: Vec<mimir::Stop> = StopPointIter::new(&mut rdr)
        .unwrap()
        .filter_map(|rc| {
            rc.map_err(|e| println!("error at csv line decoding : {}", e))
                .ok()
        })
        .collect();
    assert_eq!(stops.len(), 2);
    let mut ids: Vec<_> = stops.iter().map(|s| s.id.clone()).collect();
    assert_eq!(ids.sort(), vec!["BGT:SA:gpualsa3", "BGT:SA:bou14ju"].sort());

}
