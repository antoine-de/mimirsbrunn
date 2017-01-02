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

use std::collections::BTreeMap;
use super::AdminsVec;

pub fn format_label(admins: &AdminsVec, city_level: u32, name: &str) -> String {
    match admins.iter().position(|adm| adm.level == city_level) {
        Some(idx) => format!("{} ({})", name, admins[idx].label),
        None => name.to_string(),
    }
}

pub fn get_zip_codes_from_admins(admins: &AdminsVec) -> Vec<String> {
    let level = admins.iter().fold(0, |level, adm| {
        if adm.level > level && !adm.zip_codes.is_empty() {
            adm.level
        } else {
            level
        }
    });
    if level == 0 {
        return vec![];
    }
    admins.into_iter()
        .filter(|adm| adm.level == level)
        .flat_map(|adm| adm.zip_codes.iter().cloned())
        .collect()
}

pub fn get_way_coord(obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
                     way: &osmpbfreader::objects::Way)
                     -> mimir::Coord {
    way.nodes
        .iter()
        .filter_map(|node_id| {
            obj_map.get(&osmpbfreader::OsmId::Node(*node_id))
                .and_then(|obj| obj.node())
                .map(|node| mimir::Coord::new(node.lat(), node.lon()))
        })
        .next()
        .unwrap_or(mimir::Coord::default())
}
