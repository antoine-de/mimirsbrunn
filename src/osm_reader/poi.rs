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

use super::osm_utils::get_way_coord;
use super::osm_utils::make_centroid;
use super::OsmPbfReader;
use crate::admin_geofinder::AdminGeoFinder;
use crate::{labels, utils};
use mimir::{rubber, Poi, PoiType};
use osm_boundaries_utils::build_boundary;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeMap;
use std::error::Error;
use std::io;
use std::ops::Deref;

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
        let mut res: PoiConfig = serde_json::from_str(DEFAULT_JSON_POI_TYPES).unwrap();
        res.check().unwrap();
        res.convert_id();
        res
    }
}
impl PoiConfig {
    pub fn from_reader<R: io::Read>(r: R) -> Result<PoiConfig, Box<dyn Error>> {
        let mut res: PoiConfig = serde_json::from_reader(r)?;
        res.check()?;
        res.convert_id();
        Ok(res)
    }
    pub fn is_poi(&self, tags: &osmpbfreader::Tags) -> bool {
        self.get_poi_type(tags).is_some()
    }
    pub fn get_poi_id(&self, tags: &osmpbfreader::Tags) -> Option<&str> {
        self.get_poi_type(tags).map(|poi_type| poi_type.id.as_str())
    }
    pub fn get_poi_type(&self, tags: &osmpbfreader::Tags) -> Option<&PoiType> {
        self.rules
            .iter()
            .find(|rule| {
                rule.osm_tags_filters
                    .iter()
                    .all(|f| tags.get(&f.key).map_or(false, |v| v == &f.value))
            })
            .and_then(|rule| {
                self.poi_types
                    .iter()
                    .find(|poi_type| poi_type.id == rule.poi_type_id)
            })
    }
    pub fn check(&self) -> Result<(), Box<dyn Error>> {
        use std::collections::BTreeSet;
        let mut ids = BTreeSet::<&str>::new();
        for poi_type in &self.poi_types {
            if !ids.insert(&poi_type.id) {
                Err(format!(
                    "poi_type_id {:?} present several times",
                    poi_type.id
                ))?;
            }
        }
        for rule in &self.rules {
            if !ids.contains(rule.poi_type_id.as_str()) {
                Err(format!(
                    "poi_type_id {:?} in a rule not declared",
                    rule.poi_type_id
                ))?;
            }
        }
        Ok(())
    }
    fn convert_id(&mut self) {
        for poi_type in &mut self.poi_types {
            poi_type.id = format!("poi_type:{}", poi_type.id);
        }
        for rule in &mut self.rules {
            rule.poi_type_id = format!("poi_type:{}", rule.poi_type_id);
        }
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

fn make_properties(tags: &osmpbfreader::Tags) -> Vec<mimir::Property> {
    tags.iter()
        .map(|property| mimir::Property {
            key: property.0.to_string(),
            value: property.1.to_string(),
        })
        .collect()
}

fn parse_poi(
    osmobj: &osmpbfreader::OsmObj,
    obj_map: &BTreeMap<osmpbfreader::OsmId, osmpbfreader::OsmObj>,
    matcher: &PoiConfig,
    admins_geofinder: &AdminGeoFinder,
) -> Option<mimir::Poi> {
    let poi_type = match matcher.get_poi_type(osmobj.tags()) {
        Some(poi_type) => poi_type,
        None => {
            warn!(
                "The poi {:?} has no tags even if it passes the filters",
                osmobj.id()
            );
            return None;
        }
    };
    let (id, coord) = match *osmobj {
        osmpbfreader::OsmObj::Node(ref node) => (
            format_poi_id("node", node.id.0),
            mimir::Coord::new(node.lon(), node.lat()),
        ),
        osmpbfreader::OsmObj::Way(ref way) => {
            (format_poi_id("way", way.id.0), get_way_coord(obj_map, way))
        }
        osmpbfreader::OsmObj::Relation(ref relation) => (
            format_poi_id("relation", relation.id.0),
            make_centroid(&build_boundary(relation, obj_map)),
        ),
    };

    let name = osmobj.tags().get("name").unwrap_or(&poi_type.name);

    if coord.is_default() {
        info!(
            "The poi {} is rejected, cause: could not compute coordinates.",
            id
        );
        return None;
    }

    let adms = admins_geofinder.get(&coord);
    let zip_codes = match osmobj.tags().get("addr:postcode") {
        Some(val) if !val.is_empty() => vec![val.clone()],
        _ => utils::get_zip_codes_from_admins(&adms),
    };
    let country_codes = utils::find_country_codes(adms.iter().map(|a| a.deref()));
    Some(mimir::Poi {
        id: id,
        name: name.to_string(),
        label: labels::format_poi_label(name, adms.iter().map(|a| a.deref()), &country_codes),
        coord: coord.clone(),
        approx_coord: Some(coord.into()),
        zip_codes: zip_codes,
        administrative_regions: adms,
        weight: 0.,
        poi_type: poi_type.clone(),
        properties: make_properties(osmobj.tags()),
        address: None,
        names: mimir::I18nProperties::default(),
        labels: mimir::I18nProperties::default(),
        distance: None,
        country_codes,
    })
}

fn format_poi_id(osm_type: &str, id: i64) -> String {
    format!("poi:osm:{}:{}", osm_type, id)
}

pub fn pois(
    pbf: &mut OsmPbfReader,
    matcher: &PoiConfig,
    admins_geofinder: &AdminGeoFinder,
) -> Vec<Poi> {
    let objects = pbf.get_objs_and_deps(|o| matcher.is_poi(o.tags())).unwrap();
    objects
        .iter()
        .filter(|&(_, obj)| matcher.is_poi(obj.tags()))
        .filter_map(|(_, obj)| parse_poi(obj, &objects, matcher, admins_geofinder))
        .collect()
}

pub fn compute_poi_weight(pois_vec: &mut [Poi]) {
    for poi in pois_vec {
        for admin in &mut poi.administrative_regions {
            if admin.is_city() {
                poi.weight = admin.weight;
                break;
            }
        }
    }
}

pub fn add_address(pois_vec: &mut [Poi], rubber: &mut rubber::Rubber) {
    for poi in pois_vec {
        poi.address = rubber
            .get_address(&poi.coord)
            .ok()
            .and_then(|addrs| addrs.into_iter().next())
            .map(|addr| addr.address().unwrap());
        if poi.address.is_none() {
            warn!("The poi {:?} {:?} doesn't have address", poi.id, poi.name);
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
    fn from_str(s: &str) -> Result<PoiConfig, Box<dyn Error>> {
        PoiConfig::from_reader(io::Cursor::new(s))
    }
    #[test]
    fn default_test() {
        let c = PoiConfig::default();
        assert!(c.get_poi_id(&tags(&[])).is_none());
        for s in &[
            "college",
            "university",
            "theatre",
            "hospital",
            "post_office",
            "bicycle_rental",
            "bicycle_parking",
            "parking",
            "police",
        ] {
            assert_eq!(
                format!("poi_type:amenity:{}", s),
                c.get_poi_id(&tags(&[("amenity", s)])).unwrap()
            );
        }
        for s in &["garden", "park"] {
            assert_eq!(
                format!("poi_type:leisure:{}", s),
                c.get_poi_id(&tags(&[("leisure", s)])).unwrap()
            );
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
        from_str(
            r#"{
            "poi_types": [
                {"id": "bob", "name": "Bob"},
                {"id": "bob", "name": "Bobitto"}
            ],
            "rules": []
        }"#,
        )
        .unwrap_err();
        from_str(
            r#"{
            "poi_types": [{"id": "bob", "name": "Bob"}],
            "rules": [
                {
                    "osm_tags_filters": [{"key": "foo", "value": "bar"}],
                    "poi_type_id": "bobette"
                }
            ]
        }"#,
        )
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
        assert_eq!(
            Some("poi_type:amenity:bicycle_rental"),
            c.get_poi_id(&tags(&[("amenity:bicycle_rental", "true")]))
        );
        assert_eq!(
            Some("poi_type:amenity:parking"),
            c.get_poi_id(&tags(&[("amenity", "parking:effia")]))
        );
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
        assert_eq!(
            Some("poi_type:bob"),
            c.get_poi_id(&tags(&[
                ("bob", "bobette"),
                ("titi", "tata"),
                ("foo", "bar"),
            ],))
        );
        assert_eq!(
            Some("poi_type:titi"),
            c.get_poi_id(&tags(&[
                ("bob", "bobitta"),
                ("titi", "toto"),
                ("foo", "bar"),
            ],))
        );
        assert_eq!(
            Some("poi_type:bob_titi"),
            c.get_poi_id(&tags(&[
                ("bob", "bobette"),
                ("titi", "toto"),
                ("foo", "bar"),
            ],))
        );
        assert_eq!(
            Some("poi_type:foo"),
            c.get_poi_id(&tags(&[
                ("bob", "bobitta"),
                ("titi", "tata"),
                ("foo", "bar"),
            ],))
        );
    }
}
