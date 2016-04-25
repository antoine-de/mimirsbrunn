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
use std::collections::HashMap;
use mimirsbrunn::rubber::Rubber;
use mimirsbrunn::objects::Polygon;

pub type AdminsVec = Vec<mimirsbrunn::Admin>;

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

#[derive(Debug)]
struct AdminMatcher {
    admin_levels: HashSet<u32>,
}
impl AdminMatcher {
    pub fn new(levels: HashSet<u32>) -> AdminMatcher {
        AdminMatcher { admin_levels: levels }
    }

    pub fn is_admin(&self, obj: &osmpbfreader::OsmObj) -> bool {
        match *obj {
            osmpbfreader::OsmObj::Relation(ref rel) => {
                rel.tags.get("boundary").map_or(false, |v| v == "administrative") &&
                rel.tags.get("admin_level").map_or(false, |lvl| {
                    self.admin_levels.contains(&lvl.parse::<u32>().unwrap_or(0))
                })
            }
            _ => false,
        }
    }
}

struct BoundaryPart {
    nodes: Vec<osmpbfreader::Node>,
    first: osmpbfreader::Node,
    last: osmpbfreader::Node
}

impl BoundaryPart {
    pub fn new(nodes: Vec<osmpbfreader::Node>) -> BoundaryPart {
        BoundaryPart{first: nodes.first().unwrap().clone(), last: nodes.last().unwrap().clone(), nodes: nodes}
        //todo use lifetime
    }
}

fn get_nodes(way: &osmpbfreader::Way, objects: &HashMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>) -> Vec<osmpbfreader::Node>{
    let mut nodes = Vec::new();
    for node in &way.nodes {
        match objects.get(&osmpbfreader::OsmId::Node(*node)) {
            Some(n) => {
                match n{
                    &osmpbfreader::OsmObj::Node(ref node) => nodes.push(node.clone()),
                    _ => {}
                }
            },
            None => warn!("node not found ({}) for way {}", node, way.id)
        }
    }
    nodes
}

fn build_boundary(relation: &osmpbfreader::Relation, objects: &HashMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>) -> Option<Polygon> {
    let mut outer: Vec<osmpbfreader::Node> = Vec::new();
    let mut boundary_parts = Vec::new();
    for refe in relation.refs.iter().filter(|rf| rf.role == "outer") {
        match objects.get(&refe.member) {
            Some(m) => {
                match m{
                    &osmpbfreader::OsmObj::Way(ref way) => {
                        let mut nodes = get_nodes(&way, objects);
                        if nodes.len() < 2 {
                            continue;
                        }
                        boundary_parts.push(BoundaryPart::new(nodes));
                    }
                    _ => {}
                }
            }
            None => warn!("not found member {:?} from rel {}", refe.member, relation.id)
        }
    }
    let mut tmp = match boundary_parts.pop() {
        Some(p) => p,
        None => return None
    };
    outer.append(&mut tmp.nodes);
    let mut current = tmp.last.id;
    let mut first = tmp.first.id;
    let mut nb_try = 0;
    while current != first && nb_try < boundary_parts.len(){
        for mut next_part in &mut boundary_parts {
            if next_part.nodes.is_empty(){
                continue;
            }
            if current == next_part.first.id {
                outer.append(&mut next_part.nodes);
                current = next_part.last.id;
            }else if current == next_part.last.id {
                next_part.nodes.reverse();
                outer.append(&mut next_part.nodes);
                current = next_part.first.id;
            }

        }
        nb_try = nb_try + 1;
    }
    let polygon = Polygon::new(outer.iter().map(|n| mimirsbrunn::Coord{lat: n.lat, lon: n.lon}).collect());
    if relation.id == 105404 {
        info!("outer for {}: {:?}", relation.id, polygon.to_wkt());
    }
    Some(polygon)
}

fn administrative_regions(filename: &String, levels: HashSet<u32>) -> AdminsVec {
    let mut administrative_regions = AdminsVec::new();
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    let matcher = AdminMatcher::new(levels);
    let objects = osmpbfreader::get_objs_and_deps(&mut pbf, |o| matcher.is_admin(o)).unwrap();
    // load administratives regions
    for (_, obj) in &objects {
        if !matcher.is_admin(&obj) {
            continue;
        }
        if let &osmpbfreader::OsmObj::Relation(ref relation) = obj {
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
                Some(l) => l,
            };
            // administrative region with name ?
            let name = match relation.tags.get("name") {
                Some(val) => val,
                None => {
                    warn!("adminstrative region without name for relation {}:  admin_level {} \
                           ignored.",
                          relation.id,
                          level);
                    continue;
                }
            };
            // admininstrative region without coordinates
            let coord_centre = relation.refs
                                       .iter()
                                       .find(|rf| rf.role == "admin_centre")
                                       .and_then(|r| {
                                           objects.get(&r.member).and_then(|value| {
                                               match value {
                                                   &osmpbfreader::OsmObj::Node(ref node) => {
                                                       Some(mimirsbrunn::Coord {
                                                           lat: node.lat,
                                                           lon: node.lon,
                                                       })
                                                   }
                                                   _ => None,
                                               }
                                           })
                                       });

            let admin_id = match relation.tags.get("ref:INSEE") {
                Some(val) => format!("admin:fr:{}", val.trim_left_matches('0')),
                None => format!("admin:osm:{}", relation.id),
            };
            let zip_code = match relation.tags.get("addr:postcode") {
                Some(val) => &val[..],
                None => "",
            };
            let boundary = build_boundary(&relation, &objects);
            let admin = mimirsbrunn::Admin {
                id: admin_id,
                level: level,
                name: name.to_string(),
                zip_code: zip_code.to_string(),
                // TODO weight value ?
                weight: 1,
                coord: coord_centre,
                boundary: boundary,
            };
            administrative_regions.push(admin);
        }
    }
    return administrative_regions;
}

fn index_osm(es_cnx_string: &str, admins: &AdminsVec) -> Result<u32, rs_es::error::EsError> {
    let mut rubber = Rubber::new(es_cnx_string);
    rubber.create_index();
    match rubber.clean_db_by_doc_type(&["admin"]) {
        Err(e) => panic!("failed to clean data by document type: {}", e),
        Ok(nb) => info!("clean data by document type : {}", nb),
    }
    info!("Add data in elasticsearch db.");
    rubber.bulk_index(admins.iter())
}

fn main() {
    mimirsbrunn::logger_init().unwrap();
    debug!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let levels = args.flag_level.iter().cloned().collect();
    let res = administrative_regions(&args.flag_input, levels);
    match index_osm(&args.flag_connection_string, &res) {
        Err(e) => panic!("failed to index osm because: {}", e),
        Ok(nb) => info!("Adminstrative regions: {}", nb),
    }

}
