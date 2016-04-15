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
extern crate log;
extern crate osmpbfreader;
extern crate rustc_serialize;
extern crate docopt;
extern crate mimirsbrunn;
extern crate rs_es;

use std::collections::HashSet;
use std::collections::BTreeMap;
use osmpbfreader::OsmId;
use mimirsbrunn::rubber::Rubber;

pub type AdminsMap = BTreeMap<OsmId, mimirsbrunn::Admin>;

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_input: String,
    flag_level: Vec<u32>,
    flag_connection_string: String,
}

static USAGE: &'static str = "
Usage:
    osm2mimir --help
    osm2mimir --input=<file> [--connection-string=<connection-string>] --level=<level>...

Options:
    -h, --help            Show this message.
    -i, --input=<file>    OSM PBF file.
    -l, --level=<level>   Admin levels to keep.
    -c, --connection-string=<connection-string>
                          Elasticsearch parameters, [default: http://localhost:9200/munin]
";

fn update_coordinates(filename: &String, admins: &mut AdminsMap) {
    if admins.is_empty() {
        return;
    }
    // load coord for administratives regions
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    for obj in pbf.iter() {
        if let osmpbfreader::OsmObj::Node(ref node) = obj {
            let mut adm = match admins.get_mut(&obj.id()) {
                Some(val) => val,
                None => continue,
            };
            adm.coord.lat = node.lat;
            adm.coord.lon = node.lon;
        }
    }
}

fn administartive_regions(filename: &String, levels: &HashSet<u32>) -> AdminsMap {
    let mut administrative_regions = AdminsMap::new();
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    // load administratives regions
    for obj in pbf.iter() {
        if let osmpbfreader::OsmObj::Relation(relation) = obj {
            // not administartive region
            if !relation.tags
                        .get("boundary")
                        .map(|s| s == "administrative")
                        .unwrap_or(false) {
                continue;
            }
            let level = relation.tags
                                .get("admin_level")
                                .and_then(|s| s.parse().ok());
            let level = match level {
                None => {
                    info!("invalid admin_level for relation {}: admin_level {:?}",
                          relation.id,
                          relation.tags.get("admin_level"));
                    continue;
                }
                Some(ref l) if !levels.contains(&l) => continue,
                Some(l) => l,
            };
            // administrative region with name ?
            let name = match relation.tags.get("name") {
                Some(val) => val,
                None => {
                    info!("adminstrative region without name for relation {}:  admin_level {} \
                           ignored.",
                          relation.id,
                          level);
                    continue;
                }
            };
            // admininstrative region without coordinates
            let admin_centre = match relation.refs.iter().find(|rf| rf.role == "admin_centre") {
                Some(val) => val.member,
                None => {
                    info!("adminstrative region [{}] without coordinates for relation {}.",
                          name,
                          relation.id);
                    continue;
                }
            };

            let admin_id = match relation.tags.get("ref:INSEE") {
                Some(val) => format!("admin:fr:{}", val.trim_left_matches('0')),
                None => format!("admin:osm:{}", relation.id),
            };
            let zip_code = match relation.tags.get("addr:postcode") {
                Some(val) => &val[..],
                None => "",
            };
            let admin = mimirsbrunn::Admin {
                id: admin_id,
                level: level,
                name: name.to_string(),
                zip_code: zip_code.to_string(),
                // TODO weight value ?
                weight: 1,
                coord: mimirsbrunn::Coord {
                    lat: 0.0,
                    lon: 0.0,
                },
            };
            administrative_regions.insert(admin_centre, admin);
        }
    }
    return administrative_regions;
}

fn index_osm(es_cnx_string: &str, admins: &AdminsMap) -> Result<u32, rs_es::error::EsError> {
    let mut rubber = Rubber::new(es_cnx_string);
    rubber.create_index();
    match rubber.clean_db_by_doc_type(&["admin"]) {
        Err(e) => panic!("failed to clean data by document type: {}", e),
        Ok(nb) => info!("clean data by document type : {}", nb),
    }
    info!("Add data in elasticsearch db.");
    rubber.bulk_index(admins.values())
}

fn main() {
    mimirsbrunn::logger_init().unwrap();
    debug!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let levels = args.flag_level.iter().cloned().collect();
    let mut res = administartive_regions(&args.flag_input, &levels);
    update_coordinates(&args.flag_input, &mut res);
    match index_osm(&args.flag_connection_string, &res) {
        Err(e) => panic!("failed to index osm because: {}", e),
        Ok(nb) => info!("Adminstrative regions: {}", nb),
    }
}
