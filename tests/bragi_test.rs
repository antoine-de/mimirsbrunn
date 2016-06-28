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
use std::process::Command;
extern crate mime;
use std::collections::BTreeMap;

fn get_handler(url: String) -> rustless::Application {
    let api = bragi::api::ApiEndPoint { es_cnx_string: url }.root();
    rustless::Application::new(api)
}

pub fn bragi_tests(es_wrapper: ::ElasticSearchWrapper) {
    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    let status = Command::new(bano2mimir)
                     .args(&["--input=./tests/fixtures/sample-bano.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())])
                     .status()
                     .unwrap();
    assert!(status.success(), "`bano2mimir` failed {}", &status);
    es_wrapper.refresh();

    let handler = get_handler(format!("{}/munin", es_wrapper.host()));

    let bragi_get = |q| {
        iron_test::request::get(&format!("http://localhost:3000{}", q),
                                iron::Headers::new(),
                                &handler)
            .unwrap()
    };
    let to_json = |r| -> serde_json::Value {
        let s = iron_test::response::extract_body_to_string(r);
        serde_json::from_str(&s).unwrap()
    };
    let get_results = |r| -> Vec<_> {
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
    };

    // Call status
    let resp = bragi_get("/status");
    let result_body = iron_test::response::extract_body_to_string(resp);

    assert_eq!(result_body,
               r#"{"version":"1.1.0","es":"http://localhost:9242/munin","status":"good"}"#);

    // Call autocomplete
    let resp = bragi_get("/autocomplete?q=15 Rue Hector Malot, (Paris)");
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","#,
                         r#""geocoding":{"version":"0.1.0","query":""},"#,
                         r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
                         r#"[2.3763789999999996,48.846495],"type":"Point"},"#,
                         r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
                         r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
                         r#""name":"15 Rue Hector Malot","housenumber":"15","#,
                         r#""street":"Rue Hector Malot","postcode":null,"#,
                         r#""city":null,"administrative_regions":[]}}}]}"#);
    assert_eq!(result_body, result);

    // A(48.846431 2.376488)
    // B(48.846430 2.376306)
    // C(48.846606 2.376309)
    // D(48.846603 2.376486)
    // R(48.846495 2.376378) : 15 Rue Hector Malot, (Paris)
    // E(48.846452 2.376580) : 18 Rue Hector Malot, (Paris)


    //             E
    //
    //      A ---------------------D
    //      |                      |
    //      |         R            |
    //      |                      |
    //      |                      |
    //      B ---------------------C
    // Search with shape where house number in shape
    let mut header = iron::Headers::new();
    let mime: mime::Mime = "application/json".parse().unwrap();
    header.set(iron::headers::ContentType(mime));
    let resp = iron_test::request::post("http://localhost:3000/autocomplete?q=15 Rue Hector \
                                        Malot, (Paris)",
                                       header.clone(),
                                       r#"{"geometry":{"type":"Polygon","coordinates":[[[48.846431,2.376488],[48.846430,2.376306],[48.846606,2.376309],[ 48.846603,2.376486]]]}}"#,
                                       &handler)
                   .unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","#,
                         r#""geocoding":{"version":"0.1.0","query":""},"#,
                         r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
                         r#"[2.3763789999999996,48.846495],"type":"Point"},"#,
                         r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
                         r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
                         r#""name":"15 Rue Hector Malot","housenumber":"15","#,
                         r#""street":"Rue Hector Malot","postcode":"75012","#,
                         r#""city":null,"administrative_regions":[]}}}]}"#);
    assert_eq!(result_body, result);

    // Search with shape where house number out shape
    let resp = iron_test::request::post("http://localhost:3000/autocomplete?q=18 Rue Hector \
                                        Malot, (Paris)",
                                       header,
                                       r#"{"geometry":{"type":"Polygon","coordinates":[[[48.846431,2.376488],[48.846430,2.376306],[48.846606,2.376309],[ 48.846603,2.376486]]]}}"#,
                                       &handler)
                   .unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(r#"{"type":"FeatureCollection","geocoding":{"version":"0.1.0","query":""},"features":[]}"#);
    assert_eq!(result_body, result);

    // test with a lon/later
    // in the dataset there are 2 '20 rue hector malot', one in paris and one in trifouilli-les-ois
    let all_20 = get_results(bragi_get("/autocomplete?q=20 rue hect mal")); // in the mean time we time our prefix search_query
    assert_eq!(all_20.len(), 2);
    // the first one is paris
    // TODO uncomment this test, for the moment since osm is not loaded, the order is random
    // assert_eq!(get_labels(&all_20), vec!["20 Rue Hector Malot (Paris)", "20 Rue Hector Malot (Trifouilli-les-ois)"]);

    // if we give a lon/lat near trifouilli-les-ois, we'll have another sort
    let all_20 = get_results(bragi_get("/autocomplete?q=20 rue hect mal&lat=42.1&lon=24.2"));
    assert_eq!(get_labels(&all_20), vec!["20 Rue Hector Malot (Trifouilli-les-ois)", "20 Rue Hector Malot (Paris)"]);
}

fn get_labels<'a>(r: &'a Vec<BTreeMap<String, serde_json::Value>>) -> Vec<&'a str> {
    r.iter().map(|e| e.get("label").and_then(|l| l.as_string()).unwrap_or("")).collect()
}
