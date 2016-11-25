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
extern crate osmpbfreader;

use ::admin_geofinder::AdminGeoFinder;
use ::boundaries::{build_boundary, make_centroid};
use std::collections::{BTreeSet, BTreeMap};
use super::utils::*;
use super::OsmPbfReader;

pub type PoiTypes = BTreeMap<String, BTreeSet<String>>;
pub type PoisVec = Vec<mimir::Poi>;

#[derive(Debug)]
struct PoiMatcher {
    poi_types: PoiTypes,
}

impl PoiMatcher {
    pub fn new(types: PoiTypes) -> PoiMatcher {
        PoiMatcher { poi_types: types }
    }

    pub fn is_poi(&self, obj: &osmpbfreader::OsmObj) -> bool {
        self.poi_types.iter().any(|(poi_tag, poi_types)| {
            obj.tags().get(poi_tag).map_or(false, |poi_type| poi_types.contains(poi_type))
        })
    }
}

pub fn default_amenity_types() -> BTreeSet<String> {
    ["university",
     "hospital",
     "post_office",
     "bicycle_rental",
     "bicycle_parking",
     "parking",
     "police",
     "townhall"]
        .iter()
        .map(|&k| k.to_string())
        .collect()
}

pub fn default_leisure_types() -> BTreeSet<String> {
    ["garden", "park"]
        .iter()
        .map(|&k| k.to_string())
        .collect()
}

fn parse_poi(osmobj: &osmpbfreader::OsmObj,
             obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
             admins_geofinder: &AdminGeoFinder,
             city_level: u32)
             -> Option<mimir::Poi> {
    let (id, coord) = match *osmobj {
        osmpbfreader::OsmObj::Node(ref node) => {
            (format_poi_id("node", node.id), mimir::Coord::new(node.lat, node.lon))
        }
        osmpbfreader::OsmObj::Way(ref way) => {
            (format_poi_id("way", way.id), get_way_coord(obj_map, way))
        }
        osmpbfreader::OsmObj::Relation(ref relation) => {
            (format_poi_id("relation", relation.id),
             make_centroid(&build_boundary(&relation, &obj_map)))
        }
    };

    let name = osmobj.tags().get("name").map_or("", |name| name);
    let adms = admins_geofinder.get(&coord);
    let zip_codes = match osmobj.tags().get("addr:postcode") {
        Some(val) if !val.is_empty() => vec![val.clone()],
        _ => get_zip_codes_from_admins(&adms), 
    };
    Some(mimir::Poi {
        id: id,
        name: name.to_string(),
        label: format_label(&adms, city_level, name),
        coord: coord,
        zip_codes: zip_codes,
        administrative_regions: adms,
        weight: 1,
    })
}

fn format_poi_id(_type: &str, id: i64) -> String {
    format!("poi:osm:{}:{}", _type, id)
}

pub fn pois(pbf: &mut OsmPbfReader,
            poi_types: PoiTypes,
            admins_geofinder: &AdminGeoFinder,
            city_level: u32)
            -> PoisVec {
    let matcher = PoiMatcher::new(poi_types);
    let objects = osmpbfreader::get_objs_and_deps(pbf, |o| matcher.is_poi(o)).unwrap();
    objects.iter()
        .filter(|&(_, obj)| matcher.is_poi(&obj))
        .map(|(_, obj)| parse_poi(obj, &objects, admins_geofinder, city_level))
        .filter(|o| o.is_some())
        .map(|o| o.unwrap())
        .collect()
}

pub fn compute_poi_weight(pois_vec: &mut PoisVec, city_level: u32) {
    for poi in pois_vec {
        for admin in &mut poi.administrative_regions {
            if admin.level == city_level {
                poi.weight = admin.weight.get();
                break;
            }
        }
    }
}
