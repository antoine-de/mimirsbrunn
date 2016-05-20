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

extern crate docopt;
extern crate csv;
extern crate rustc_serialize;
extern crate curl;
extern crate mimir;
#[macro_use]
extern crate log;

use std::path::Path;
use mimir::rubber::Rubber;
use std::fs;

#[derive(RustcDecodable, RustcEncodable)]
pub struct Bano {
    pub id: String,
    pub nb: String,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub src: String,
    pub lat: f64,
    pub lon: f64,
}

impl Bano {
    pub fn insee(&self) -> &str {
        assert!(self.id.len() >= 5);
        self.id[..5].trim_left_matches('0')
    }
    pub fn fantoir(&self) -> &str {
        assert!(self.id.len() >= 10);
        &self.id[..10]
    }
    pub fn into_addr(self) -> mimir::Addr {
        let street_name = format!("{}, {} {}", self.street, self.zip, self.city);
        let addr_name = format!("{} {}", self.nb, street_name);
        let street_id = format!("street:{}", self.fantoir().to_string());
        let admin = mimir::Admin {
            id: format!("admin:fr:{}", self.insee()),
            level: 8,
            name: self.city,
            zip_code: self.zip,
            weight: 1,
            coord: Some(mimir::Coord {
                lat: 0.0,
                lon: 0.0,
            }),
            boundary: None,
        };
        let street = mimir::Street {
            id: street_id,
            street_name: self.street,
            name: street_name,
            administrative_region: Some(admin),
            weight: 1,
        };
        mimir::Addr {
            id: format!("addr:{};{}", self.lat, self.lon),
            house_number: self.nb,
            street: street,
            name: addr_name,
            coord: mimir::Coord {
                lat: self.lat,
                lon: self.lon,
            },
            weight: 1,
        }
    }
}

fn index_bano<I>(cnx_string: &str, dataset: &str, files: I)
    where I: Iterator<Item = std::path::PathBuf>
{
    let doc_type = "addr";
    let mut rubber = Rubber::new(cnx_string);
    let addr_index = rubber.make_index(doc_type, dataset).unwrap();
    info!("Add data in elasticsearch db.");
    for f in files {
        info!("importing {:?}...", &f);
        let mut rdr = csv::Reader::from_file(&f).unwrap().has_headers(false);

        let iter = rdr.decode().map(|r| {
            let b: Bano = r.unwrap();
            b.into_addr()
        });
        match rubber.bulk_index(&addr_index, iter) {
            Err(e) => panic!("failed to bulk insert file {:?} because: {}", &f, e),
            Ok(nb) => info!("importing {:?}: {} addresses added.", &f, nb),
        }
    }
    rubber.publish_index(doc_type, dataset, addr_index).unwrap();
}

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_input: String,
    flag_connection_string: String,
    flag_dataset: String,
}

static USAGE: &'static str = "
Usage:
    bano2mimir --input=<input> [--connection-string=<connection-string>]

    -i, --input=<input>           Bano files. Can be either a directory or a file.
    -c, --connection-string=<connection-string>
                                  Elasticsearch parameters, [default: http://localhost:9200/munin]
    -d, --dataset=<dataset>       Name of the dataset, [default: fr]
";

fn main() {
    mimir::logger_init().unwrap();
    info!("importing bano into Mimir");

    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let file_path = Path::new(&args.flag_input);
    if file_path.is_dir() {
        let paths: std::fs::ReadDir = fs::read_dir(&args.flag_input).unwrap();
        index_bano(&args.flag_connection_string,
                   &args.flag_dataset,
                   paths.map(|p| p.unwrap().path()));
    } else {
        index_bano(&args.flag_connection_string,
                   &args.flag_dataset,
                   std::iter::once(std::path::PathBuf::from(&args.flag_input)));
    }
}
