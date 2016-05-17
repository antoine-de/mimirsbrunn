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
extern crate chrono;

extern crate mimirsbrunn;
extern crate serde;


use std::collections::HashSet;
use mimir::rubber::Rubber;

pub type AdminsVec = Vec<mimir::Admin>;
pub type StreetsVec = Vec<mimir::Street>;
pub type OsmPbfReader = osmpbfreader::OsmPbfReader<std::fs::File>;

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_input: String,
    flag_level: Vec<u32>,
    flag_connection_string: String,
    flag_way: bool,
    flag_dataset: String,
}

static USAGE: &'static str = "
Usage:
    osm2mimir --help
    osm2mimir --input=<file> [--connection-string=<connection-string>] --level=<level> ... [--way]

Options:
    -h, --help            Show this message.
    -i, --input=<file>    OSM PBF file.
    -l, --level=<level>   Admin levels to keep.
    -w, --way             Import ways
    -c, --connection-string=<connection-string>
                          Elasticsearch parameters, [default: http://localhost:9200/munin]
    -d, --dataset=<dataset>         Name of the dataset, [default: fr]
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


fn parse_osm_pbf(path: &str) -> OsmPbfReader {
    let path = std::path::Path::new(&path);
    osmpbfreader::OsmPbfReader::new(std::fs::File::open(&path).unwrap())
}

fn administrative_regions(pbf: &mut OsmPbfReader, levels: HashSet<u32>) -> AdminsVec {
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
            let boundary = mimirsbrunn::boundaries::build_boundary(&relation, &objects);
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

fn streets(pbf: &mut OsmPbfReader) -> StreetsVec {

    let is_valid_obj = |obj: &osmpbfreader::OsmObj| -> bool {
        match *obj {
            osmpbfreader::OsmObj::Way(ref way) => {
                way.tags.get("highway").map_or(false, |x| !x.is_empty()) &&
                way.tags.get("name").map_or(false, |x| !x.is_empty())
            }
            osmpbfreader::OsmObj::Relation(ref rel) => {
                rel.tags.get("type").map_or(false, |v| v == "associatedStreet")
            }
            _ => false,
        }
    };

    let mut objs_map = osmpbfreader::get_objs_and_deps(pbf, is_valid_obj).unwrap();
    // Sometimes, streets can be devided into several "way"s that still have the same street name.
    // The reason why a street is devided may be that a part of the street become a bridge/tunne/etc.
    // In this case, a "relation" tagged with (type = associatedStreet) is used to group all these "way"s.
    // In order not to have doublons in autocompleion, we should keep only one "way" in the relation
    // and remove all the rest way whose street name is the same.
    let mut objs_to_remove = Vec::<osmpbfreader::OsmId>::new();

    for (_, rel_obj) in &objs_map {
        if let &osmpbfreader::OsmObj::Relation(ref rel) = rel_obj {
            let mut found = false;
            for ref_obj in &rel.refs {
                if let &osmpbfreader::OsmId::Way(_) = &ref_obj.member {
                    if !found {
                        found = true;
                        continue;
                    }
                    objs_to_remove.push(ref_obj.member.clone());
                };
            }
        };
    }

    for osm_id in objs_to_remove {
        objs_map.remove(&osm_id);
    }

    objs_map.iter()
            .filter_map(|(_, obj)| {
                if let &osmpbfreader::OsmObj::Way(ref way) = obj {
                    way.tags.get("name").and_then(|way_name| {
                        Some(mimir::Street {
                            id: way.id.to_string(),
                            street_name: way_name.to_string(),
                            name: way_name.to_string(),
                            weight: 1,
                            administrative_region: None,
                        })
                    })
                } else {
                    None
                }
            })
            .collect()

}
fn index<C>(es_cnx_string: &str, items: C, index_name: &str) -> Result<u32, rs_es::error::EsError>
        where C: IntoIterator,
              C::Item: serde::Serialize + mimir::DocType,
{
    let mut rubber = Rubber::new_with_index(es_cnx_string, index_name);
    rubber.bulk_index(items.into_iter())
}


fn init_indexes(es_cnx_string: &str, type_: &str, dataset: &str,) -> Result<String, String> {
    let mut rubber = Rubber::new(es_cnx_string);
    let index_name = format!("{}_{}", type_, dataset);
    if !rubber.is_existing_index(&index_name).unwrap() {
        rubber.create_index_with_name(&index_name.to_string());
        try!(rubber.alias(&type_.to_string(), &vec![index_name.clone()], &vec![]));
        try!(rubber.alias(&"munin".to_string(), &vec![type_.to_string()], &vec![]));
    }
    Ok(index_name)
}

fn main() {
    mimir::logger_init().unwrap();
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let levels = args.flag_level.iter().cloned().collect();
    let mut parsed_pbf = parse_osm_pbf(&args.flag_input);
    debug!("creation of indexes");
    let index_name = init_indexes(&args.flag_connection_string, &"admin", &args.flag_dataset).unwrap();

    debug!("importing adminstrative region into Mimir");
    let nb_admins = index(&args.flag_connection_string, administrative_regions(&mut parsed_pbf, levels), &index_name).unwrap();
    info!("Nb of indexed admin: {}", nb_admins);

    if args.flag_way {
        let index_name = init_indexes(&args.flag_connection_string, &"way", &args.flag_dataset).unwrap();
        let nb_street = index(&args.flag_connection_string, streets(&mut parsed_pbf), &index_name).unwrap();
        info!("Nb of indexed street: {}", nb_street);
    }
}
