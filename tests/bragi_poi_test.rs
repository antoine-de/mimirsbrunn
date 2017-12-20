// Copyright © 2017, Canal TP and/or its affiliates. All rights reserved.
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

extern crate bragi;
extern crate iron_test;
extern crate serde_json;
use serde_json::Map;
use serde_json::Value;
use super::BragiHandler;
use super::get_values;
use super::get_value;
use super::get_types;
use super::count_types;
use super::filter_by_type;
use mimir::{Poi, MimirObject};


pub fn bragi_poi_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // ******************************************
    // We load the OSM dataset (with the POIs) and three-cities bano dataset
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf (including ways and pois)
    // - bano-three_cities
    // ******************************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(
        osm2mimir,
        vec![
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-way".into(),
            "--import-poi".into(),
            "--level=8".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    ::launch_and_assert(
        bano2mimir,
        vec![
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    poi_admin_address_test(&bragi);
    poi_admin_test(&bragi);
    poi_zip_code_test(&bragi);
    poi_from_osm_test(&bragi);
    poi_misspelt_one_word_admin_test(&bragi);
}


fn poi_admin_address_test(bragi: &BragiHandler) {
    let geocodings = bragi.get("/autocomplete?q=Le-Mée-sur-Seine Courtilleraies");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);

    // the first element returned should be the poi 'Le-Mée-sur-Seine Courtilleraies'
    let poi = geocodings.first().unwrap();
	let properties = poi.get("properties").and_then(|json| json.as_array()).unwrap();
	assert_eq!(properties.len(), 9);
    assert_eq!(get_value(poi, "type"), Poi::doc_type());
    assert_eq!(get_value(poi, "label"), "Le-Mée-sur-Seine Courtilleraies");
    assert_eq!(get_value(poi, "postcode"), "77350");
}

fn poi_admin_test(bragi: &BragiHandler) {
    // with this search we should be able to find a poi called Melun Rp
    let geocodings = bragi.get("/autocomplete?q=Melun Rp");
    let types = get_types(&geocodings);
    let count = count_types(&types, Poi::doc_type());
    assert!(count >= 1);

    assert!(get_values(&geocodings, "label").contains(
        &"Melun Rp (Melun)",
    ));

    // when we search for just 'Melun', we should find some places in melun
    let geocodings = bragi.get("/autocomplete?q=Melun");
    for postcodes in get_values(&geocodings, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    // we should also be able to find the city of melun which will carry more postcodes
    let cities = filter_by_type(&geocodings, "city");
    assert_eq!(cities.len(), 1);
    let melun = &cities.first().unwrap();
    assert_eq!(get_value(melun, "name"), "Melun");
    assert_eq!(get_value(melun, "postcode"), "77000;77003;77008;CP77001");
    assert_eq!(get_value(melun, "label"), "Melun (77000-CP77001)");
}

fn poi_zip_code_test(bragi: &BragiHandler) {
    // search by zip code
    let geocodings = bragi.get("/autocomplete?q=77000&limit=15");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 2);
    assert_eq!(count_types(&types, "city"), 3);
    assert_eq!(count_types(&types, "street"), 7);

    // search by zip code and limit is string type
    let geocodings = bragi.raw_get("/autocomplete?q=77000&limit=ABCD");
    assert!(geocodings.is_err(), true);

    // search by zip code and limit < 0
    let geocodings = bragi.raw_get("/autocomplete?q=77000&limit=-1");
    assert!(geocodings.is_err(), true);

    // search by zip code and limit and offset
    let all_20 = bragi.get("/autocomplete?q=77000&limit=10&offset=0");
    assert_eq!(all_20.len(), 10);

    let all_20 = bragi.get("/autocomplete?q=77000&limit=10&offset=10");
    assert_eq!(all_20.len(), 2);
}

fn get_poi_type_ids(e: &Map<String, Value>) -> Vec<&str> {
    let array = match e.get("poi_types").and_then(|json| json.as_array()) {
        None => return vec![],
        Some(array) => array,
    };
    array
        .iter()
        .filter_map(|v| v.as_object().and_then(|o| o.get("id")))
        .filter_map(|o| o.as_str())
        .collect()
}

fn poi_from_osm_test(bragi: &BragiHandler) {
    // search poi: Poi as a relation in osm data
    let geocodings = bragi.get("/autocomplete?q=Parking (Le Coudray-Montceaux)");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);
    let first_poi = geocodings
        .iter()
        .find(|e| get_value(e, "type") == Poi::doc_type())
        .unwrap();
    assert_eq!(get_value(first_poi, "citycode"), "91179");
    assert_eq!(get_poi_type_ids(first_poi), &["poi_type:amenity:parking"]);

    // search poi: Poi as a way in osm data
    let geocodings = bragi.get("/autocomplete?q=77000 Hôtel de Ville (Melun)");
    let types = get_types(&geocodings);
    assert!(count_types(&types, Poi::doc_type()) >= 1);
    let poi = geocodings.first().unwrap();
    assert_eq!(get_value(poi, "type"), Poi::doc_type());
    assert_eq!(get_value(poi, "id"), "poi:osm:way:112361498");
    assert_eq!(get_value(poi, "label"), "Hôtel de Ville (Melun)");
    assert_eq!(get_poi_type_ids(poi), &["poi_type:amenity:townhall"]);

    // we search for POIs with a type but an empty name, we should have set the name with the type.
    // for exemple there are parkings without name (but with the tag "anemity" = "Parking"),
    // we should be able to query them
    let geocodings = bragi.get("/autocomplete?q=Parking");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 8);

    // we search for a POI (id = 2561223) with a label but an empty coord
    // (barycenter not computed so far), it should be filtered.
    let geocodings = bragi.get("/autocomplete?q=ENSE3 site Ampère");
    // we can find other results (due to the fuzzy search, but we can't find the 'site Ampère')
    assert!(!get_values(&geocodings, "label").contains(
        &"ENSE3 site Ampère",
    ));
}

fn poi_misspelt_one_word_admin_test(bragi: &BragiHandler) {
    // with this search we should be able to find a poi called "Melun"
    let geocodings = bragi.get("/autocomplete?q=Melun");
    let types = get_types(&geocodings);
    let count = count_types(&types, Poi::doc_type());
    assert!(count >= 1);
    assert!(get_values(&geocodings, "label").contains(
        &"Melun Rp (Melun)",
    ));

    // when we search for 'Meluuun', we should find some places in melun
    let geocodings = bragi.get("/autocomplete?q=Meluuun");
    for postcodes in get_values(&geocodings, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    // we should also be able to find the city of melun which will carry more postcodes
    let cities = filter_by_type(&geocodings, "city");
    assert_eq!(cities.len(), 1);
    let melun = &cities.first().unwrap();
    assert_eq!(get_value(melun, "name"), "Melun");
}
