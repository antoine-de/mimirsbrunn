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

use super::get_values;
use super::to_json;
use super::BragiHandler;
use iron_test;
use serde_json::json;

pub fn bragi_bano_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // *********************************
    // We load bano files
    // *********************************
    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    crate::launch_and_assert(
        bano2mimir,
        vec![
            "--input=./tests/fixtures/sample-bano.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    status_test(&bragi);
    simple_bano_autocomplete_test(&bragi);
    simple_bano_shape_filter_test(&bragi);
    simple_bano_lon_lat_test(&bragi);
    long_bano_address_test(&bragi);
    reverse_bano_test(&bragi);
}

fn status_test(bragi: &BragiHandler) {
    let resp = bragi.raw_get("/status").unwrap();
    assert_eq!(to_json(resp).pointer("/status"), Some(&json!("good")));
}

fn simple_bano_autocomplete_test(bragi: &BragiHandler) {
    let resp = bragi
        .raw_get("/autocomplete?q=15 Rue Hector Malot, (Paris)")
        .unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(
        r#"{"type":"FeatureCollection","#,
        r#""geocoding":{"version":"0.1.0","query":""},"#,
        r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
        r#"[2.376379,48.846495],"type":"Point"},"#,
        r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
        r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
        r#""name":"15 Rue Hector Malot","housenumber":"15","#,
        r#""street":"Rue Hector Malot","postcode":"75012","#,
        r#""city":null,"citycode":null,"#,
        r#""administrative_regions":[]}}}]}"#
    );
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
    let shape = r#"{"shape":{"type":"Feature","geometry":{"type":"Polygon",
        "coordinates":[[[2.376488, 48.846431],
        [2.376306, 48.846430],[2.376309, 48.846606],[ 2.376486, 48.846603]]]}}}"#;
    let resp = bragi
        .raw_post_shape("/autocomplete?q=15 Rue Hector Malot, (Paris)", shape)
        .unwrap();

    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(
        r#"{"type":"FeatureCollection","#,
        r#""geocoding":{"version":"0.1.0","query":""},"#,
        r#""features":[{"type":"Feature","geometry":{"coordinates":"#,
        r#"[2.376379,48.846495],"type":"Point"},"#,
        r#""properties":{"geocoding":{"id":"addr:2.376379;48.846495","#,
        r#""type":"house","label":"15 Rue Hector Malot (Paris)","#,
        r#""name":"15 Rue Hector Malot","housenumber":"15","#,
        r#""street":"Rue Hector Malot","postcode":"75012","#,
        r#""city":null,"citycode":null,"#,
        r#""administrative_regions":[]}}}]}"#
    );
    assert_eq!(result_body, result);

    // Search with shape where house number out of shape
    let resp = bragi
        .raw_post_shape("/autocomplete?q=18 Rue Hector Malot, (Paris)", shape)
        .unwrap();
    let result_body = iron_test::response::extract_body_to_string(resp);
    let result = concat!(
        r#"{"type":"FeatureCollection","#,
        r#""geocoding":{"version":"0.1.0","query":""},"features":[]}"#
    );
    assert_eq!(result_body, result);
}

fn simple_bano_lon_lat_test(bragi: &BragiHandler) {
    // test with a lon/lat priorisation
    // in the dataset there are two '20 rue hector malot',
    // one in paris and one in trifouilli-les-Oies
    // in the mean time we test our prefix search_query
    let all_20 = bragi.get("/autocomplete?q=20 rue hect mal");
    assert_eq!(all_20.len(), 2);
    // the first one is paris (since Paris has more streets, it is prioritized first)
    // TODO uncomment this test, for the moment since osm is not loaded, the order is random
    // assert_eq!(get_labels(&all_20),
    //            vec!["20 Rue Hector Malot (Paris)", "20 Rue Hector Malot (Trifouilli-les-Oies)"]);

    // if we give a lon/lat near trifouilli-les-Oies, we'll have another sort
    let all_20 = bragi.get("/autocomplete?q=20 rue hector malot&lat=50.2&lon=2.0");
    assert_eq!(
        get_values(&all_20, "label"),
        vec![
            "20 Rue Hector Malot (Trifouilli-les-Oies)",
            "20 Rue Hector Malot (Paris)",
        ]
    );
    // and when we're in paris, we get paris first
    let all_20 = bragi.get("/autocomplete?q=20 rue hector malot&lat=48&lon=2.4");
    assert_eq!(
        get_values(&all_20, "label"),
        vec![
            "20 Rue Hector Malot (Paris)",
            "20 Rue Hector Malot (Trifouilli-les-Oies)",
        ]
    );
}

fn long_bano_address_test(bragi: &BragiHandler) {
    // test with a very long request which consists of an exact address and something else
    // and the "something else" should not disturb the research
    let all_20 = bragi.get(
        "/autocomplete?q=The Marvellous Navitia Developers Kisio Digital 20 rue hector \
         malot paris",
    );
    assert_eq!(all_20.len(), 1);
    assert_eq!(
        get_values(&all_20, "label"),
        vec!["20 Rue Hector Malot (Paris)"]
    );
}

fn reverse_bano_test(bragi: &BragiHandler) {
    let res = bragi.get("/reverse?lon=2.37716&lat=48.8468");
    assert_eq!(res.len(), 1);
    assert_eq!(
        get_values(&res, "label"),
        vec!["20 Rue Hector Malot (Paris)"]
    );

    let res = bragi.get("/reverse?lon=1.3787628&lat=43.6681995");
    assert_eq!(
        get_values(&res, "label"),
        vec!["2 Rue des Pins (Beauzelle)"]
    );
}
