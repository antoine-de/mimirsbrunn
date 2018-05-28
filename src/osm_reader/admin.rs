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

extern crate mimir;
extern crate osm_boundaries_utils;
extern crate osmpbfreader;

use self::osm_boundaries_utils::build_boundary;
use super::OsmPbfReader;
use admin_geofinder::AdminGeoFinder;
use itertools::Itertools;
use osm_reader::osm_utils::make_centroid;
use std::sync::RwLock;
use std::collections::BTreeSet;
pub type StreetsVec = Vec<mimir::Street>;
use cosmogony::ZoneType;

#[derive(Debug)]
pub struct AdminMatcher {
    admin_levels: BTreeSet<u32>,
}

impl AdminMatcher {
    pub fn new(levels: BTreeSet<u32>) -> AdminMatcher {
        AdminMatcher {
            admin_levels: levels,
        }
    }

    pub fn is_admin(&self, obj: &osmpbfreader::OsmObj) -> bool {
        match *obj {
            osmpbfreader::OsmObj::Relation(ref rel) => {
                rel.tags
                    .get("boundary")
                    .map_or(false, |v| v == "administrative")
                    && rel.tags.get("admin_level").map_or(false, |lvl| {
                        self.admin_levels.contains(&lvl.parse::<u32>().unwrap_or(0))
                    })
            }
            _ => false,
        }
    }
}

pub fn read_administrative_regions(
    pbf: &mut OsmPbfReader,
    levels: BTreeSet<u32>,
    city_level: u32,
) -> Vec<mimir::Admin> {
    let mut administrative_regions = Vec::<mimir::Admin>::new();
    let mut insee_inserted = BTreeSet::default();
    let matcher = AdminMatcher::new(levels);
    info!("reading pbf...");
    let objects = pbf.get_objs_and_deps(|o| matcher.is_admin(o)).unwrap();
    info!("reading pbf done.");
    // load administratives regions
    for obj in objects.values() {
        if !matcher.is_admin(obj) {
            continue;
        }
        if let osmpbfreader::OsmObj::Relation(ref relation) = *obj {
            let level = relation
                .tags
                .get("admin_level")
                .and_then(|s| s.parse().ok());
            let level = match level {
                None => {
                    warn!(
                        "relation/{} ({}): invalid admin_level: {:?}, skipped",
                        relation.id.0,
                        relation.tags.get("name").map_or("", String::as_str),
                        relation.tags.get("admin_level")
                    );
                    continue;
                }
                Some(l) => l,
            };
            // administrative region with name ?
            let name = match relation.tags.get("name") {
                Some(val) => val,
                None => {
                    warn!(
                        "relation/{}: adminstrative region without name, skipped",
                        relation.id.0
                    );
                    continue;
                }
            };

            // admininstrative region without coordinates
            let coord_center = relation
                .refs
                .iter()
                .find(|r| r.role == "admin_centre")
                .and_then(|r| objects.get(&r.member))
                .and_then(|o| o.node())
                .map(|node| mimir::Coord::new(node.lon(), node.lat()));
            let (admin_id, insee_id) = match read_insee(&relation.tags) {
                Some(val) if !insee_inserted.contains(val) => {
                    insee_inserted.insert(val.to_string());
                    (format!("admin:fr:{}", val), val)
                }
                Some(val) => {
                    let id = format!("admin:osm:{}", relation.id.0);
                    warn!(
                        "relation/{}: have the INSEE {} that is already used, using {} as id",
                        relation.id.0, val, id
                    );
                    (id, val)
                }
                None => (format!("admin:osm:{}", relation.id.0), ""),
            };

            let zip_codes = read_zip_codes(&relation.tags);
            let boundary = build_boundary(relation, &objects);
            let zone_type = get_zone_type(level, city_level);
            let admin_type = if zone_type == Some(ZoneType::City) {
                mimir::AdminType::City
            } else {
                mimir::AdminType::Unknown
            };

            let weight = relation
                .tags
                .get("population")
                .and_then(|p| p.parse().ok())
                .or_else(|| {
                    let rel = relation.refs.iter().find(|r| r.role == "admin_centre")?;
                    objects
                        .get(&rel.member)?
                        .node()?
                        .tags
                        .get("population")?
                        .parse()
                        .ok()
                })
                .unwrap_or(0.);

            let admin = mimir::Admin {
                id: admin_id,
                insee: insee_id.to_string(),
                level: level,
                name: name.to_string(),
                label: format!("{}{}", name.to_string(), format_zip_codes(&zip_codes)),
                zip_codes: zip_codes,
                weight: RwLock::new(0.),
                coord: coord_center.unwrap_or_else(|| make_centroid(&boundary)),
                boundary: boundary,
                admin_type: admin_type,
                zone_type: zone_type,
            };
            administrative_regions.push(admin);
        }
    }

    compute_admin_weight(&mut administrative_regions);

    administrative_regions
}

fn get_zone_type(level: u32, city_lvl: u32) -> Option<ZoneType> {
    if level == city_lvl {
        Some(ZoneType::City)
    } else {
        None
    }
}

pub fn compute_admin_weight(admins: &mut [mimir::Admin]) {
    let max = admins.iter().fold(1f64, |m, a| f64::max(m, a.weight.get()));
    for ref mut a in admins {
        a.weight.set(a.weight.get() / max);
    }
}

pub fn format_zip_codes(zip_codes: &[String]) -> String {
    match zip_codes.len() {
        0 => "".to_string(),
        1 => format!(" ({})", zip_codes.first().unwrap()),
        _ => format!(
            " ({}-{})",
            zip_codes.first().unwrap(),
            zip_codes.last().unwrap()
        ),
    }
}

pub fn read_zip_codes(tags: &osmpbfreader::Tags) -> Vec<String> {
    let zip_code = tags
        .get("addr:postcode")
        .or_else(|| tags.get("postal_code"))
        .map_or("", |val| &val[..]);
    zip_code
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .sorted()
}

pub fn read_insee(tags: &osmpbfreader::Tags) -> Option<&str> {
    tags.get("ref:INSEE").map(|v| v.trim_left_matches('0'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_correct_admin_type() {
        assert_eq!(
            get_zone_type(1 /*level*/, 1 /*city level*/),
            Some(ZoneType::City)
        );
        assert_eq!(get_zone_type(2, 1), None);
    }
}
