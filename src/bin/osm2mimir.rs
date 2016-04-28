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
extern crate mimir;
extern crate rs_es;

use std::collections::HashSet;
use std::collections::HashMap;
use mimirsbrunn::rubber::Rubber;
use mimirsbrunn::objects::{Polygon, MultiPolygon};

pub type AdminsVec = Vec<mimir::Admin>;
pub type StreetsVec = Vec<mimir::Street>;
pub type ParsedPbf = osmpbfreader::OsmPbfReader<std::fs::File>;

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
}

impl BoundaryPart {
    pub fn new(nodes: Vec<osmpbfreader::Node>) -> BoundaryPart {
        BoundaryPart { nodes: nodes }
    }
    pub fn first(&self) -> i64 {
        self.nodes.first().unwrap().id
    }
    pub fn last(&self) -> i64 {
        self.nodes.last().unwrap().id
    }
}

fn get_nodes(way: &osmpbfreader::Way,
             objects: &HashMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>)
             -> Vec<osmpbfreader::Node> {
    way.nodes
       .iter()
       .filter_map(|node_id| objects.get(&osmpbfreader::OsmId::Node(*node_id)))
       .filter_map(|node_obj| if let &osmpbfreader::OsmObj::Node(ref node) = node_obj {
           Some(node.clone())
       } else {
           None
       })
       .collect()
}

fn build_boundary(relation: &osmpbfreader::Relation,
                  objects: &HashMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>)
                  -> Option<MultiPolygon> {
    let mut boundary_parts: Vec<BoundaryPart> =
        relation.refs
                .iter()
                .filter(|rf| rf.role == "outer" || rf.role == "" || rf.role == "enclave")
                .filter_map(|refe| {
                    objects.get(&refe.member).or_else(|| {
                        warn!("missing element for relation {}", relation.id);
                        None
                    })
                })
                .filter_map(|way_obj| if let &osmpbfreader::OsmObj::Way(ref way) = way_obj {
                    Some(way)
                } else {
                    None
                })
                .map(|way| get_nodes(&way, objects))
                .filter(|nodes| nodes.len() > 1)
                .map(|nodes| BoundaryPart::new(nodes))
                .collect();
    let mut multipoly = MultiPolygon { polygons: Vec::new() };
    // we want to try build a polygon for a least each way
    while !boundary_parts.is_empty() {
        let mut current = -1;
        let mut first = 0;
        let mut nb_try = 0;

        let mut outer: Vec<osmpbfreader::Node> = Vec::new();
        // we try to close the polygon, if we can't we want to at least have tried one time per
        // way. We could improve that latter by trying to attach the way to both side of the
        // polygon
        let max_try = boundary_parts.len();
        while current != first && nb_try < max_try {
            let mut i = 0;
            while i < boundary_parts.len() {
                if outer.is_empty() {
                    // our polygon is empty, we initialise it with the current way
                    first = boundary_parts[i].first();
                    current = boundary_parts[i].last();
                    outer.append(&mut boundary_parts[i].nodes);
                    // this way has been used, we remove it from the pool
                    boundary_parts.remove(i);
                    continue;
                }
                if current == boundary_parts[i].first() {
                    // the start of current way touch the polygon, we add it and remove it from the
                    // pool
                    current = boundary_parts[i].last();
                    outer.append(&mut boundary_parts[i].nodes);
                    boundary_parts.remove(i);
                } else if current == boundary_parts[i].last() {
                    // the end of the current way touch the polygon, we reverse the way and add it
                    current = boundary_parts[i].first();
                    boundary_parts[i].nodes.reverse();
                    outer.append(&mut boundary_parts[i].nodes);
                    boundary_parts.remove(i);
                } else {
                    i += 1;
                    // didnt do anything, we want to explore the next way, if we had do something we
                    // will have removed the current way and there will be no need to increment
                }
                if current == first {
                    // our polygon is closed, we create it and add it to the multipolygon
                    let polygon = Polygon::new(outer.iter()
                                                    .map(|n| {
                                                        mimir::Coord {
                                                            lat: n.lat,
                                                            lon: n.lon,
                                                        }
                                                    })
                                                    .collect());
                    multipoly.polygons.push(polygon);
                    break;
                }
            }
            nb_try += 1;
        }
    }
    debug!("polygon for relation {};{}", relation.id, multipoly.to_wkt());
    if multipoly.polygons.is_empty() {
        None
    } else {
        Some(multipoly)
    }
}

fn parse_osm_pbf(path: &String) -> ParsedPbf {
    let path = std::path::Path::new(&path);
    osmpbfreader::OsmPbfReader::new(std::fs::File::open(&path).unwrap())
}

fn administrative_regions(pbf: &mut ParsedPbf, levels: HashSet<u32>) -> AdminsVec {
    let mut administrative_regions = AdminsVec::new();
    let matcher = AdminMatcher::new(levels);
    let objects = osmpbfreader::get_objs_and_deps(pbf, |o| matcher.is_admin(o)).unwrap();
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
                                                       Some(mimir::Coord {
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
            let admin = mimir::Admin {
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

fn streets(pbf: &mut ParsedPbf) -> StreetsVec {

    let is_valid_street = |obj: &osmpbfreader::OsmObj| -> bool {
        match *obj {
            osmpbfreader::OsmObj::Way(ref way) => {
                way.tags.get("highway").map_or(false, |x: &String| !x.is_empty()) &&
                way.tags.get("name").map_or(false, |x: &String| !x.is_empty())
            }
            _ => false,
        }
    };

    let is_associated_street_relation = |obj: &osmpbfreader::OsmObj| -> bool {
        match *obj {
            osmpbfreader::OsmObj::Relation(ref rel) => {
                rel.tags.get("type").map_or(false, |v| v == "associatedStreet")
            }
            _ => false,
        }
    };

    let mut street_map = osmpbfreader::get_objs_and_deps(pbf, is_valid_street).unwrap();
    let relation_map = osmpbfreader::get_objs_and_deps(pbf, is_associated_street_relation).unwrap();

    for (_, rel_obj) in &relation_map {
        if let &osmpbfreader::OsmObj::Relation(ref rel) = rel_obj {
            let mut found = false;
            for ref_obj in &rel.refs {
                if let &osmpbfreader::OsmId::Way(_) = &ref_obj.member {
                    if !found {
                        found = true;
                        continue;
                    };
                    street_map.remove(&ref_obj.member);
                };
            }
        };
    }

    street_map.iter()
              .filter_map(|(osm_id, obj)| {
                  if let &osmpbfreader::OsmObj::Way(ref way) = obj {
                      if let &osmpbfreader::OsmId::Way(ref way_id) = osm_id {
                          way.tags.get("name").and_then(|way_name| {
                              Some(mimirsbrunn::Street {
                                  id: way_id.to_string(),
                                  street_name: way_name.to_string(),
                                  name: way_name.to_string(),
                                  weight: 1,
                                  administrative_region: None,
                              })
                          })
                      } else {
                          None
                      }
                  } else {
                      None
                  }
              })
              .collect()

}

fn index_osm(es_cnx_string: &str, admins: &AdminsVec, streets: &StreetsVec) {
    let mut rubber = Rubber::new(es_cnx_string);
    rubber.create_index();
    match rubber.clean_db_by_doc_type(&["admin", "street"]) {
        Err(e) => panic!("failed to clean data by document type: {}", e),
        Ok(nb) => info!("clean data by document type : {}", nb),
    }
    info!("Add data in elasticsearch db.");
    match rubber.bulk_index(admins.iter()) {
        Err(e) => panic!("failed to index admins of osm because: {}", e),
        Ok(nb) => info!("Nb of indexed adminstrative regions: {}", nb),
    }

    match rubber.bulk_index(streets.iter()) {
        Err(e) => panic!("failed to index streets of osm because: {}", e),
        Ok(nb) => info!("Nb of indexed streets: {}", nb),
    }
}

fn main() {
    mimir::logger_init().unwrap();
    debug!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let levels = args.flag_level.iter().cloned().collect();
    let mut parsed_pbf = parse_osm_pbf(&args.flag_input);
    let admins = administrative_regions(&mut parsed_pbf, levels);
    let streets = streets(&mut parsed_pbf);
    index_osm(&args.flag_connection_string, &admins, &streets);
}
