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
use std::process::Command;

fn get_handler(url: String) -> rustless::Application {
    let api = bragi::api::ApiEndPoint{es_cnx_string: url}.root();
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
    
    // Call status
    let resp = iron_test::request::get("http://localhost:3000/status",
                                iron::Headers::new(),
                                &handler).unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    
    assert_eq!(result_body, r#"{"version":"1.0.0","es":"http://localhost:9242/munin","status":"good"}"#);
	
    // Call autocomplete
    let resp = iron_test::request::get("http://localhost:3000/autocomplete?q=15 Rue Hector Malot, (Paris)",
                                iron::Headers::new(),
                                &handler).unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = vec![r#"{"Autocomplete":{"type":"FeatureCollection","geocoding":{"version":"0.1.0","query":""},"#,
                  r#""features":[{"type":"Feature","geometry":{"coordinates":[2.3763789999999996,48.846495],"type":"Point"},"#,
                  r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","type":"house","label":"15 Rue Hector Malot, (Paris)","#,
                  r#""name":"15 Rue Hector Malot, (Paris)","housenumber":"15","street":"Rue Hector Malot, (Paris)","postcode":null,"city":null}}}]}}"#];
    assert_eq!(result_body, result.join(""));
}
