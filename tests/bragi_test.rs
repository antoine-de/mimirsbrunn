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

extern crate bragi;
extern crate rustless;
extern crate iron;
extern crate iron_test;
extern crate serde_json;
extern crate mime;
use std::collections::BTreeMap;
use serde_json::Value;

fn status_test(bragi: &BragiHandler) {
    let resp = bragi.get("/status");
    assert_eq!(to_json(resp).find("status"),
               Some(&Value::String("good".to_string())));
}

fn simple_bano_autocomplete_test(bragi: &BragiHandler) {
    let resp = bragi.get("/autocomplete?q=15 Rue Hector Malot, (Paris)");
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","#,
                         r#""geocoding":{"version":"0.1.0","query":""},"#,
                         r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
                         r#"[2.376379,48.846495],"type":"Point"},"#,
                         r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
                         r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
                         r#""name":"15 Rue Hector Malot","housenumber":"15","#,
                         r#""street":"Rue Hector Malot","postcode":"75012","#,
                         r#""city":null,"city_code":null,"citycode":null,"level":null,"#,
                         r#""administrative_regions":[]}}}]}"#);
    assert_eq!(result_body, result);
}

// A(48.846431 2.376488)
// B(48.846430 2.376306)
// C(48.846606 2.376309)
// D(48.846603 2.376486)
// R(48.846495 2.376378) : 15 Rue Hector Malot, (Paris)
// E(48.846452 2.376580) : 18 Rue Hector Malot, (Paris)
//
//             E
//
//      A ---------------------D
//      |                      |
//      |         R            |
//      |                      |
//      |                      |
//      B ---------------------C
fn simple_bano_shape_filter_test(bragi: &BragiHandler) {
    // Search with shape where house number in shape
    let shape = r#"{"geometry":{"type":"Polygon","coordinates":[[[2.376488, 48.846431],
        [2.376306, 48.846430],[2.376309, 48.846606],[ 2.376486, 48.846603]]]}}"#;
    let resp = bragi.post_shape("/autocomplete?q=15 Rue Hector Malot, (Paris)", shape);

    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","#,
                         r#""geocoding":{"version":"0.1.0","query":""},"#,
                         r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
                         r#"[2.376379,48.846495],"type":"Point"},"#,
                         r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
                         r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
                         r#""name":"15 Rue Hector Malot","housenumber":"15","#,
                         r#""street":"Rue Hector Malot","postcode":"75012","#,
                         r#""city":null,"city_code":null,"citycode":null,"level":null,"#,
                         r#""administrative_regions":[]}}}]}"#);
    assert_eq!(result_body, result);

    // Search with shape where house number out of shape
    let resp = bragi.post_shape("/autocomplete?q=18 Rue Hector Malot, (Paris)", shape);
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","#,
                         r#""geocoding":{"version":"0.1.0","query":""},"features":[]}"#);
    assert_eq!(result_body, result);
}

fn simple_bano_lon_lat_test(bragi: &BragiHandler) {
    // test with a lon/lat priorisation
    // in the dataset there are two '20 rue hector malot', one in paris and one in trifouilli-les-Oies
    // in the mean time we time our prefix search_query
    let all_20 = get_results(bragi.get("/autocomplete?q=20 rue hect mal"));
    assert_eq!(all_20.len(), 2);
    // the first one is paris (since Paris has more streets, it is prioritized first)
    // TODO uncomment this test, for the moment since osm is not loaded, the order is random
    // assert_eq!(get_labels(&all_20),
    //            vec!["20 Rue Hector Malot (Paris)", "20 Rue Hector Malot (Trifouilli-les-Oies)"]);

    // if we give a lon/lat near trifouilli-les-Oies, we'll have another sort
    let all_20 = get_results(bragi.get("/autocomplete?q=20 rue hector malot&lat=50.2&lon=2.0"));
    assert_eq!(get_values(&all_20, "label"),
               vec!["20 Rue Hector Malot (Trifouilli-les-Oies)", "20 Rue Hector Malot (Paris)"]);
    // and when we're in paris, we get paris first
    let all_20 = get_results(bragi.get("/autocomplete?q=20 rue hector malot&lat=48&lon=2.4"));
    assert_eq!(get_values(&all_20, "label"),
               vec!["20 Rue Hector Malot (Paris)", "20 Rue Hector Malot (Trifouilli-les-Oies)"]);
}


fn zip_code_test(bragi: &BragiHandler) {
    let all_20 = get_results(bragi.get("/autocomplete?q=77000"));
    assert_eq!(all_20.len(), 10);
    for postcodes in get_values(&all_20, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    assert!(get_values(&all_20, "postcode").iter().any(|r| *r == "77000;77003;77008;CP77001"));

    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 7);

    let count = count_types(&types, "city");
    assert_eq!(count, 3);

    let count = count_types(&types, "house");
    assert_eq!(count, 0);
}

fn zip_code_street_test(bragi: &BragiHandler) {
    let all_20 = get_results(bragi.get("/autocomplete?q=77000 Lotissement le Clos de Givry"));
    assert_eq!(all_20.len(), 1);
    assert!(get_values(&all_20, "postcode").iter().all(|r| *r == "77000"));

    let boundary = all_20[0]["administrative_regions"].pointer("/0/boundary").unwrap();
    assert!(boundary.is_null());

    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 1);
    let first_street = all_20.iter().find(|e| get_value(e, "type") == "street");
    assert_eq!(get_value(first_street.unwrap(), "citycode"), "77255");

    let count = count_types(&types, "city");
    assert_eq!(count, 0);

    let count = count_types(&types, "house");
    assert_eq!(count, 0);
}

fn zip_code_admin_test(bragi: &BragiHandler) {
    let all_20 = get_results(bragi.get("/autocomplete?q=77000 Vaux-le-Pénil"));
    assert_eq!(all_20.len(), 4);
    assert!(get_values(&all_20, "postcode").iter().all(|r| *r == "77000"));
    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 3);

    let count = count_types(&types, "city");
    assert_eq!(count, 1);
    let first_city = all_20.iter().find(|e| get_value(e, "type") == "city");
    assert_eq!(get_value(first_city.unwrap(), "citycode"), "77487");

    let count = count_types(&types, "house");
    assert_eq!(count, 0);
}

// zip_code on addr

fn three_cities_housenumber_zip_code_test(bragi: &BragiHandler) {
    // we search for a house number with a postcode, we should be able to find
    // the house number with this number in this city
    let all_20 = get_results(bragi.get("/autocomplete?q=3 rue 77255"));
    assert_eq!(all_20.len(), 1);
    assert!(get_values(&all_20, "postcode").iter().all(|r| *r == "77255"));
    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 0);

    let count = count_types(&types, "city");
    assert_eq!(count, 0);

    let count = count_types(&types, "house");
    assert_eq!(count, 1);
    let first_house = all_20.iter().find(|e| get_value(e, "type") == "house");
    assert_eq!(get_value(first_house.unwrap(), "citycode"), "77255");
    assert_eq!(get_value(first_house.unwrap(), "label"),
               "3 Rue du Four à Chaux (Livry-sur-Seine)");
}

fn three_cities_zip_code_test(bragi: &BragiHandler) {
    // we query with only a zip code, we should be able to find admins,
    // and some street of it (and all on this admin)
    let res = get_results(bragi.get("/autocomplete?q=77000"));
    assert_eq!(res.len(), 10);
    assert!(get_values(&res, "postcode").iter().all(|r| r.contains("77000")));
    let types = get_types(&res);
    // since we did not ask for an house number, we should get none
    assert_eq!(count_types(&types, "house"), 0);
}

fn three_cities_zip_code_address_test(bragi: &BragiHandler) {
    let all_20 = get_results(bragi.get("/autocomplete?q=77288 2 Rue de la Reine Blanche"));
    assert_eq!(all_20.len(), 1);
    assert!(get_values(&all_20, "postcode").iter().all(|r| *r == "77288"));
    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 0);

    let count = count_types(&types, "city");
    assert_eq!(count, 0);

    let count = count_types(&types, "house");
    assert_eq!(count, 1);

    assert_eq!(get_values(&all_20, "label"),
               vec!["2 Rue de la Reine Blanche (Melun)"]);
}

fn three_cities_shape_test(bragi: &BragiHandler) {
    //      A ---------------------D
    //      |                      |
    //      |        === street    |
    //      |                      |
    //      |                      |
    //      B ---------------------C
    // Search with shape where street in shape
    let shape = r#"{"geometry":{"type":"Polygon", "coordinates": [[[2.656546, 48.537227],
        [2.657608, 48.537244],[2.656476, 48.536545],[2.657340, 48.536602]]]}}"#;

    let geocodings = get_results(bragi.post_shape("/autocomplete?q=Rue du Port", shape));
    assert_eq!(geocodings.len(), 1);
    assert_eq!(get_values(&geocodings, "label"),
               vec!["Rue du Port (Melun)"]);

    //      A ---------------------D
    //      |                      |
    //      |                      | === street
    //      |                      |
    //      |                      |
    //      B ---------------------C
    // Search with shape where street outside shape
    let shape = r#"{"geometry":{"type":"Polygon","coordinates":[[[2.656546, 68.537227],
        [2.657608, 68.537244],[2.656476, 68.536545],[2.657340, 68.536602]]]}}"#;

    let geocodings = get_results(bragi.post_shape("/autocomplete?q=Rue du Port", shape));
    assert_eq!(geocodings.len(), 0);

    //      A ---------------------D
    //      |                      |
    //      |        X Melun       |
    //      |                      |
    //      |                      |
    //      B ---------------------C
    // Search with shape where admin in shape
    let shape = r#"{"geometry":{"type":"Polygon","coordinates":[[[2.656546, 48.538927]
        ,[2.670816, 48.538927],[2.676476, 48.546545],[2.656546, 48.546545]]]}}"#;

    let geocodings = get_results(bragi.post_shape("/autocomplete?q=Melun", shape));
    assert_eq!(geocodings.len(), 1);
    assert_eq!(get_values(&geocodings, "name"), vec!["Melun"]);

    //      A ---------------------D
    //      |                      |
    //      |                      | X Melun
    //      |                      |
    //      |                      |
    //      B ---------------------C
    // Search with shape where admin outside shape
    let shape = r#"{"geometry":{"type":"Polygon","coordinates":[[[2.656546, 66.538927],
        [2.670816, 68.538927],[2.676476, 68.546545],[2.656546, 68.546545]]]}}"#;

    let geocodings = get_results(bragi.post_shape("/autocomplete?q=Melun", shape));
    assert_eq!(geocodings.len(), 0);
}

fn poi_admin_address_test(bragi: &BragiHandler) {
    let geocodings = get_results(bragi.get("/autocomplete?q=Le-Mée-sur-Seine Courtilleraies"));
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, "poi"), 1);

    // the first element returned should be the poi 'Le-Mée-sur-Seine Courtilleraies'
    let poi = geocodings.first().unwrap();
    assert_eq!(get_value(poi, "type"), "poi");
    assert_eq!(get_value(poi, "label"), "Le-Mée-sur-Seine Courtilleraies");
    assert_eq!(get_value(poi, "postcode"), "77350");
}

fn poi_admin_test(bragi: &BragiHandler) {
    // with this search we should be able to find a poi called Melun Rp
    let geocodings = get_results(bragi.get("/autocomplete?q=Melun Rp"));
    let types = get_types(&geocodings);
    let count = count_types(&types, "poi");
    assert!(count >= 1);

    assert!(get_values(&geocodings, "label").contains(&"Melun Rp (Melun)"));

    // when we search for just 'Melun', we should find some places in melun
    let geocodings = get_results(bragi.get("/autocomplete?q=Melun"));
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
    let geocodings = get_results(bragi.get("/autocomplete?q=77000&limit=15"));
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, "poi"), 2);
    assert_eq!(count_types(&types, "city"), 3);
    assert_eq!(count_types(&types, "street"), 7);

    // search by zip code and limit is string type
    let geocodings = bragi.params_validation("/autocomplete?q=77000&limit=ABCD");
    assert!(geocodings.is_err(), true);

    // search by zip code and limit < 0
    let geocodings = bragi.params_validation("/autocomplete?q=77000&limit=-1");
    assert!(geocodings.is_err(), true);

    // search by zip code and limit and offset
    let all_20 = get_results(bragi.get("/autocomplete?q=77000&limit=10&offset=0"));
    assert_eq!(all_20.len(), 10);

    let all_20 = get_results(bragi.get("/autocomplete?q=77000&limit=10&offset=10"));
    assert_eq!(all_20.len(), 2);
}

fn poi_from_osm_test(bragi: &BragiHandler) {
    // search poi: Poi as a relation in osm data
    let geocodings = get_results(bragi.get("/autocomplete?q=Parking (Le Coudray-Montceaux)"));
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, "poi"), 1);
    let first_poi = geocodings.iter().find(|e| get_value(e, "type") == "poi");
    assert_eq!(get_value(first_poi.unwrap(), "citycode"), "91179");

    // search poi: Poi as a way in osm data
    let geocodings = get_results(bragi.get("/autocomplete?q=77000 Hôtel de Ville (Melun)"));
    let types = get_types(&geocodings);
    assert!(count_types(&types, "poi") >= 1);
    let poi = geocodings.first().unwrap();
    assert_eq!(get_value(poi, "type"), "poi");
    assert_eq!(get_value(poi, "id"), "poi:osm:way:112361498");
    assert_eq!(get_value(poi, "label"), "Hôtel de Ville (Melun)");

    // we search for POIs with a type but an empty name, we should have set the name with the type.
    // for exemple there are parkings without name (but with the tag "anemity" = "Parking"),
    // we should be able to query them
    let geocodings = get_results(bragi.get("/autocomplete?q=Parking"));
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, "poi"), 5);

    // we search for a POI (id = 2561223) with a label but an empty ?, it should be filtered)
    let geocodings = get_results(bragi.get("/autocomplete?q=ENSE3 site Ampère"));
    // we can find other results (due to the fuzzy search, but we can't find the 'site Ampère')
    assert!(!get_values(&geocodings, "label").contains(&"ENSE3 site Ampère"));
}

fn stop_attached_to_admin_test(bragi: &BragiHandler) {
    // with this query we should find only one response, a stop
    let response = get_results(bragi.get("/autocomplete?q=14 juillet"));
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "14 Juillet");
    assert_eq!(get_value(stop, "name"), "14 Juillet");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:second_station");
    assert_eq!(get_value(stop, "citycode"), "77487");

    // this stop area is in the boundary of the admin 'Vaux-le-Pénil',
    // it should have been associated to it
    assert_eq!(get_value(stop, "city"), "Vaux-le-Pénil");
    let admins = stop.get("administrative_regions").and_then(|a| a.as_array());
    assert_eq!(admins.map(|a| a.len()).unwrap_or(0), 1);
}

fn stop_no_admin_test(bragi: &BragiHandler) {
    // we query another stop, but this one is outside the range of an admin,
    // we should get the stop, but with no admin attached to it
    let response = get_results(bragi.get("/autocomplete?q=Far west station"));
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "Far west station");
    assert_eq!(get_value(stop, "name"), "Far west station");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:station_no_city");
    assert_eq!(get_value(stop, "city"), "");
    let admins = stop.get("administrative_regions").and_then(|a| a.as_array());
    assert_eq!(admins.map(|a| a.len()).unwrap_or(0), 0);
}

pub fn bragi_tests(es_wrapper: ::ElasticSearchWrapper) {

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);

    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // *********************************
    // We load bano files
    // *********************************
    ::launch_and_assert(bano2mimir,
                        vec!["--input=./tests/fixtures/sample-bano.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    status_test(&bragi);
    simple_bano_autocomplete_test(&bragi);
    simple_bano_shape_filter_test(&bragi);
    simple_bano_lon_lat_test(&bragi);

    // *********************************
    // We then load the OSM dataset
    // the current dataset are thus:
    // - sample-bano
    // - osm_fixture.osm.pbf
    // *********************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    info!("Launching {}", osm2mimir);
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
                             "--import-way".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    zip_code_test(&bragi);
    zip_code_street_test(&bragi);
    zip_code_admin_test(&bragi);

    // *********************************
    // We then load another bano dataset
    // the current dataset are thus:
    // - sample-bano
    // - bano-three_cities
    // - osm_fixture.osm.pbf
    // *********************************
    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    ::launch_and_assert(bano2mimir,
                        vec!["--input=./tests/fixtures/bano-three_cities.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    three_cities_housenumber_zip_code_test(&bragi);
    three_cities_zip_code_test(&bragi);
    three_cities_zip_code_address_test(&bragi);
    three_cities_shape_test(&bragi);

    // ******************************************
    // We then load the OSM dataset with the POIs
    // the current dataset are thus:
    // - sample-bano
    // - bano-three_cities
    // - osm_fixture.osm.pbf (including pois)
    // ******************************************
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
                             "--import-poi".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    poi_admin_address_test(&bragi);
    poi_admin_test(&bragi);
    poi_zip_code_test(&bragi);
    poi_from_osm_test(&bragi);

    // ******************************************
    // we then load a stop file
    // the current dataset are thus:
    // - sample-bano
    // - bano-three_cities
    // - osm_fixture.osm.pbf (including pois)
    // - stops.txt
    // ******************************************
    let stops2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    info!("Launching {}", stops2mimir);
    ::launch_and_assert(stops2mimir,
                        vec!["--input=./tests/fixtures/stops.txt".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    stop_attached_to_admin_test(&bragi);
    stop_no_admin_test(&bragi);
}

pub struct BragiHandler {
    app: rustless::Application,
}

impl BragiHandler {
    pub fn new(url: String) -> BragiHandler {
        let api = bragi::api::ApiEndPoint { es_cnx_string: url }.root();
        BragiHandler { app: rustless::Application::new(api) }
    }

    pub fn params_validation(&self, q: &str) -> iron::IronResult<iron::Response> {
        iron_test::request::get(&format!("http://localhost:3000{}", q),
                                iron::Headers::new(),
                                &self.app)
    }

    pub fn get(&self, q: &str) -> iron::Response {
        self.params_validation(q).unwrap()
    }

    pub fn post_shape(&self, q: &str, shape: &str) -> iron::Response {
        let mut header = iron::Headers::new();
        let mime: mime::Mime = "application/json".parse().unwrap();
        header.set(iron::headers::ContentType(mime));

        iron_test::request::post(&format!("http://localhost:3000{}", q),
                                 header,
                                 shape,
                                 &self.app)
            .unwrap()
    }
}

fn to_json(r: iron::Response) -> Value {
    let s = iron_test::response::extract_body_to_string(r);
    serde_json::from_str(&s).unwrap()
}

fn get_results(r: iron::Response) -> Vec<BTreeMap<String, Value>> {
    to_json(r)
        .find("features")
        .expect("wrongly formated bragi response")
        .as_array()
        .expect("features must be array")
        .iter()
        .map(|f| {
            f.pointer("/properties/geocoding")
                .expect("no geocoding object in bragi response")
                .as_object()
                .unwrap()
                .clone()
        })
        .collect()
}

fn get_values<'a>(r: &'a [BTreeMap<String, Value>], val: &'a str) -> Vec<&'a str> {
    r.iter().map(|e| get_value(e, val)).collect()
}

fn get_value<'a>(e: &'a BTreeMap<String, Value>, val: &'a str) -> &'a str {
    e.get(val).and_then(|l| l.as_str()).unwrap_or("")
}

fn get_types(r: &[BTreeMap<String, Value>]) -> Vec<&str> {
    r.iter().map(|e| e.get("type").and_then(|l| l.as_str()).unwrap_or("")).collect()
}

fn filter_by_type<'a>(r: &'a [BTreeMap<String, Value>],
                      t: &'a str)
                      -> Vec<BTreeMap<String, Value>> {
    r.iter()
        .filter(|e| e.get("type").and_then(|l| l.as_str()).unwrap_or("") == t)
        .cloned()
        .collect()
}

fn count_types(types: &[&str], value: &str) -> usize {
    types.iter().filter(|&t| *t == value).count()
}
