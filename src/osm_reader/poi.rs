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
use std::io;
use std::error::Error;
use serde_json;
use admin_geofinder::AdminGeoFinder;
use boundaries::{build_boundary, make_centroid};
use utils::{format_label, get_zip_codes_from_admins};
use super::osm_utils::get_way_coord;
use super::OsmPbfReader;
use mimir::Poi;

#[derive(Serialize, Deserialize, Debug)]
struct PoiType {
    id: String,
    name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct OsmTagsFilter {
    key: String,
    value: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    osm_tags_filters: Vec<OsmTagsFilter>,
    poi_type_id: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PoiConfig {
    poi_types: Vec<PoiType>,
    rules: Vec<Rule>,
}
impl Default for PoiConfig {
    fn default() -> Self {
        let res: PoiConfig = serde_json::from_str(DEFAULT_JSON_POI_TYPES).unwrap();
        res.check().unwrap();
        res
    }
}
impl PoiConfig {
    pub fn from_reader<R: io::Read>(r: R) -> Result<PoiConfig, Box<Error>> {
        let res: PoiConfig = try!(serde_json::from_reader(r));
        try!(res.check());
        Ok(res)
    }
    pub fn get_poi_id(&self, tags: &osmpbfreader::Tags) -> Option<&str> {
        self.rules
            .iter()
            .find(|rule| {
                rule.osm_tags_filters
                    .iter()
                    .all(|f| tags.get(&f.key).map_or(false, |v| v == &f.value))
            })
            .map(|rule| rule.poi_type_id.as_str())
    }
    pub fn check(&self) -> Result<(), Box<Error>> {
        use std::collections::BTreeSet;
        let mut ids = BTreeSet::<&str>::new();
        for poi_type in &self.poi_types {
            if !ids.insert(&poi_type.id) {
                try!(Err(format!("poi_type_id {:?} present several times", poi_type.id)));
            }
        }
        for rule in &self.rules {
            if !ids.contains(rule.poi_type_id.as_str()) {
                try!(Err(format!("poi_type_id {:?} in a rule not declared", rule.poi_type_id)));
            }
        }
        Ok(())
    }
}
const DEFAULT_JSON_POI_TYPES: &'static str = r#"
{
  "poi_types": [
    {"id": "amenity:college", "name": "École"},
    {"id": "amenity:university", "name": "Université"},
    {"id": "amenity:theatre", "name": "Théâtre"},
    {"id": "amenity:hospital", "name": "Hôpital"},
    {"id": "amenity:post_office", "name": "Bureau de poste"},
    {"id": "amenity:bicycle_rental", "name": "Station VLS"},
    {"id": "amenity:bicycle_parking", "name": "Parking vélo"},
    {"id": "amenity:parking", "name": "Parking"},
    {"id": "amenity:police", "name": "Police, gendarmerie"},
    {"id": "amenity:townhall", "name": "Mairie"},
    {"id": "leisure:garden", "name": "Jardin"},
    {"id": "leisure:park", "name": "Parc, espace vert"}
  ],
  "rules": [
    {
      "osm_tags_filters": [{"key": "amenity", "value": "college"}],
      "poi_type_id": "amenity:college"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "university"}],
      "poi_type_id": "amenity:university"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "theatre"}],
      "poi_type_id": "amenity:theatre"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "hospital"}],
      "poi_type_id": "amenity:hospital"
    },
   {
      "osm_tags_filters": [{"key": "amenity", "value": "post_office"}],
      "poi_type_id": "amenity:post_office"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "bicycle_rental"}],
      "poi_type_id": "amenity:bicycle_rental"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "bicycle_parking"}],
      "poi_type_id": "amenity:bicycle_parking"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "parking"}],
      "poi_type_id": "amenity:parking"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "police"}],
      "poi_type_id": "amenity:police"
    },
    {
      "osm_tags_filters": [{"key": "amenity", "value": "townhall"}],
      "poi_type_id": "amenity:townhall"
    },
    {
      "osm_tags_filters": [{"key": "leisure", "value": "garden"}],
      "poi_type_id": "leisure:garden"
    },
    {
      "osm_tags_filters": [{"key": "leisure", "value": "park"}],
      "poi_type_id": "leisure:park"
    }
  ]
}
"#;

fn parse_poi(osmobj: &osmpbfreader::OsmObj,
             obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
             admins_geofinder: &AdminGeoFinder,
             city_level: u32)
             -> Option<mimir::Poi> {
    let (id, coord) = match *osmobj {
        osmpbfreader::OsmObj::Node(ref node) => {
            (format_poi_id("node", node.id.0), mimir::Coord::new(node.lat(), node.lon()))
        }
        osmpbfreader::OsmObj::Way(ref way) => {
            (format_poi_id("way", way.id.0), get_way_coord(obj_map, way))
        }
        osmpbfreader::OsmObj::Relation(ref relation) => {
            (format_poi_id("relation", relation.id.0),
             make_centroid(&build_boundary(relation, obj_map)))
        }
    };

    let name = osmobj.tags().get("name").or_else(|| osmobj.tags().get("amenity"));

    if coord.is_default() {
        info!("The poi {} is rejected, cause: could not compute coordinates.",
              id);
        return None;
    }

    let name = if let Some(s) = name {
        s
    } else {
        info!("The poi {} is rejected, cause: could not compute name.", id);
        return None;
    };

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
            matcher: &PoiConfig,
            admins_geofinder: &AdminGeoFinder,
            city_level: u32)
            -> Vec<Poi> {
    let objects = pbf.get_objs_and_deps(|o| matcher.get_poi_id(o.tags()).is_some()).unwrap();
    objects.iter()
        .filter(|&(_, obj)| matcher.get_poi_id(obj.tags()).is_some())
        .filter_map(|(_, obj)| parse_poi(obj, &objects, admins_geofinder, city_level))
        .collect()
}

pub fn compute_poi_weight(pois_vec: &mut [Poi], city_level: u32) {
    for poi in pois_vec {
        for admin in &mut poi.administrative_regions {
            if admin.level == city_level {
                poi.weight = admin.weight.get();
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;
    fn tags(v: &[(&str, &str)]) -> osmpbfreader::Tags {
        v.iter().map(|&(k, v)| (k.into(), v.into())).collect()
    }
    fn from_str(s: &str) -> Result<PoiConfig, Box<Error>> {
        PoiConfig::from_reader(io::Cursor::new(s))
    }
    #[test]
    fn default_test() {
        let c = PoiConfig::default();
        assert!(c.get_poi_id(&tags(&[])).is_none());
        for s in &["college",
                   "university",
                   "theatre",
                   "hospital",
                   "post_office",
                   "bicycle_rental",
                   "bicycle_parking",
                   "parking",
                   "police"] {
            assert_eq!(format!("amenity:{}", s),
                       c.get_poi_id(&tags(&[("amenity", s)])).unwrap());
        }
        for s in &["garden", "park"] {
            assert_eq!(format!("leisure:{}", s),
                       c.get_poi_id(&tags(&[("leisure", s)])).unwrap());
        }
    }
    #[test]
    fn parsing_errors() {
        from_str("").unwrap_err();
        from_str("{}").unwrap_err();
        from_str("42").unwrap_err();
        from_str("{").unwrap_err();
        from_str(r#"{"poi_types": []}"#).unwrap_err();
        from_str(r#"{"rules": []}"#).unwrap_err();
        from_str(r#"{"poi_types": [], "rules": []}"#).unwrap();
        from_str(r#"{"poi_types": [{"id": "foo"}], "rules": []}"#).unwrap_err();
        from_str(r#"{"poi_types": [{"name": "bar"}], "rules": []}"#).unwrap_err();
        from_str(r#"{"poi_types": [{"id": "foo", "name": "bar"}], "rules": []}"#).unwrap();
    }
    #[test]
    fn check_tests() {
        from_str(r#"{
            "poi_types": [
                {"id": "bob", "name": "Bob"},
                {"id": "bob", "name": "Bobitto"}
            ],
            "rules": []
        }"#)
            .unwrap_err();
        from_str(r#"{
            "poi_types": [{"id": "bob", "name": "Bob"}],
            "rules": [
                {
                    "osm_tags_filters": [{"key": "foo", "value": "bar"}],
                    "poi_type_id": "bobette"
                }
            ]
        }"#)
            .unwrap_err();
    }
    #[test]
    fn check_with_colon() {
        let json = r#"{
            "poi_types": [
                {"id": "amenity:bicycle_rental", "name": "Station VLS"},
                {"id": "amenity:parking", "name": "Parking"}
            ],
            "rules": [
                {
                    "osm_tags_filters": [
                        {"key": "amenity:bicycle_rental", "value": "true"}
                    ],
                    "poi_type_id": "amenity:bicycle_rental"
                },
                {
                    "osm_tags_filters": [
                        {"key": "amenity", "value": "parking:effia"}
                    ],
                    "poi_type_id": "amenity:parking"
                }
            ]
        }"#;
        let c = from_str(json).unwrap();
        assert_eq!(Some("amenity:bicycle_rental"),
                   c.get_poi_id(&tags(&[("amenity:bicycle_rental", "true")])));
        assert_eq!(Some("amenity:parking"),
                   c.get_poi_id(&tags(&[("amenity", "parking:effia")])));
    }
    #[test]
    fn check_all_tags_first_match() {
        let json = r#"{
            "poi_types": [
                {"id": "bob_titi", "name": "Bob is Bobette and Titi is Toto"},
                {"id": "bob", "name": "Bob is Bobette"},
                {"id": "titi", "name": "Titi is Toto"},
                {"id": "foo", "name": "Foo is Bar"}
            ],
            "rules": [
                {
                    "osm_tags_filters": [
                        {"key": "bob", "value": "bobette"},
                        {"key": "titi", "value": "toto"}
                    ],
                    "poi_type_id": "bob_titi"
                },
                {
                    "osm_tags_filters": [
                        {"key": "bob", "value": "bobette"}
                    ],
                    "poi_type_id": "bob"
                },
                {
                    "osm_tags_filters": [
                        {"key": "titi", "value": "toto"}
                    ],
                    "poi_type_id": "titi"
                },
                {
                    "osm_tags_filters": [
                        {"key": "foo", "value": "bar"}
                    ],
                    "poi_type_id": "foo"
                }
            ]
        }"#;
        let c = from_str(json).unwrap();
        assert_eq!(Some("bob"),
                   c.get_poi_id(&tags(&[("bob", "bobette"), ("titi", "tata"), ("foo", "bar")])));
        assert_eq!(Some("titi"),
                   c.get_poi_id(&tags(&[("bob", "bobitta"), ("titi", "toto"), ("foo", "bar")])));
        assert_eq!(Some("bob_titi"),
                   c.get_poi_id(&tags(&[("bob", "bobette"), ("titi", "toto"), ("foo", "bar")])));
        assert_eq!(Some("foo"),
                   c.get_poi_id(&tags(&[("bob", "bobitta"), ("titi", "tata"), ("foo", "bar")])));
    }
}
