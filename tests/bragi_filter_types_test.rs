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
use super::BragiHandler;
use super::{count_types, get_types, get_value};
use serde_json::json;

pub fn bragi_filter_types_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let mut bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // ******************************************
    // we the OSM dataset, three-cities bano dataset and a stop file
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf
    // - bano-three_cities
    // - stops.txt
    // ******************************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    crate::launch_and_assert(
        osm2mimir,
        vec![
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-way".into(),
            "--import-admin".into(),
            "--import-poi".into(),
            "--level=8".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    crate::launch_and_assert(
        bano2mimir,
        vec![
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let stops2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    crate::launch_and_assert(
        stops2mimir,
        vec![
            "--input=./tests/fixtures/stops.txt".into(),
            "--dataset=dataset1".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    no_type_no_dataset_test(&mut bragi);
    type_stop_area_no_dataset_test(&mut bragi);
    type_poi_and_dataset_test(&mut bragi);
    type_poi_and_city_no_dataset_test(&mut bragi);
    type_poi_and_city_with_percent_encoding_no_dataset_test(&mut bragi);
    type_stop_area_dataset_test(&mut bragi);
    unvalid_type_test(&mut bragi);
    addr_by_id_test(&mut bragi);
    admin_by_id_test(&mut bragi);
    street_by_id_test(&mut bragi);
    stop_by_id_test(&mut bragi);
    stop_area_that_does_not_exists(&mut bragi);
    stop_area_invalid_index(&mut bragi);
}

fn no_type_no_dataset_test(bragi: &mut BragiHandler) {
    // with this query we should not find any stops
    let response = bragi.get("/autocomplete?q=Parking vélo Saint-Martin");
    let types = get_types(&response);
    let count = count_types(&types, "public_transport:stop_area");
    assert_eq!(count, 0);
}

fn type_stop_area_no_dataset_test(bragi: &mut BragiHandler) {
    // with this query we should return an empty response
    let response =
        bragi.get("/autocomplete?q=Parking vélo Saint-Martin&type[]=public_transport:stop_area");
    assert!(response.is_empty());
}

fn type_poi_and_dataset_test(bragi: &mut BragiHandler) {
    // with this query we should only find pois
    let response =
        bragi.get("/autocomplete?q=Parking vélo Saint-Martin&pt_dataset[]=dataset1&type[]=poi");
    let types = get_types(&response);
    assert_eq!(count_types(&types, "public_transport:stop_area"), 0);
    assert_eq!(count_types(&types, "city"), 0);
    assert_eq!(count_types(&types, "street"), 0);
    assert_eq!(count_types(&types, "house"), 0);
    assert!(count_types(&types, "poi") > 0);

    let poi = response.first().unwrap();
    assert_eq!(get_value(poi, "name"), "Parking vélo");
}

fn type_poi_and_city_no_dataset_test(bragi: &mut BragiHandler) {
    // with this query we should only find pois and cities
    let response = bragi.get("/autocomplete?q=melun&type[]=poi&type[]=city");
    let types = get_types(&response);
    assert_eq!(count_types(&types, "public_transport:stop_area"), 0);
    assert_eq!(count_types(&types, "street"), 0);
    assert_eq!(count_types(&types, "house"), 0);
    assert!(count_types(&types, "city") > 0);
    assert!(count_types(&types, "poi") > 0);
}

fn type_poi_and_city_with_percent_encoding_no_dataset_test(bragi: &mut BragiHandler) {
    // Same test as before but with percent encoded type param
    let response = bragi.get("/autocomplete?q=melun&type%5B%5D=poi&type%5B%5D=city");
    let types = get_types(&response);
    assert_eq!(count_types(&types, "public_transport:stop_area"), 0);
    assert_eq!(count_types(&types, "street"), 0);
    assert_eq!(count_types(&types, "house"), 0);
    assert!(count_types(&types, "city") > 0);
    assert!(count_types(&types, "poi") > 0);
}

fn type_stop_area_dataset_test(bragi: &mut BragiHandler) {
    // with this query we should only find stop areas
    let response = bragi.get(
        "/autocomplete?q=Vaux-le-Pénil&pt_dataset[]=dataset1&type[]=public_transport:\
         stop_area",
    );
    let types = get_types(&response);
    assert!(count_types(&types, "public_transport:stop_area") > 0);
    assert_eq!(count_types(&types, "street"), 0);
    assert_eq!(count_types(&types, "house"), 0);
    assert_eq!(count_types(&types, "city"), 0);
    assert_eq!(count_types(&types, "poi"), 0);
}

fn unvalid_type_test(bragi: &mut BragiHandler) {
    assert_eq!(
        bragi.get_unchecked_json("/autocomplete?q=melun&type[]=unvalid"),
        (
            actix_web::http::StatusCode::BAD_REQUEST,
            json!({
                "short": "validation error",
                "long": "invalid argument: failed with reason: unknown variant `unvalid`, expected one of `city`, `house`, `poi`, `public_transport:stop_area`, `street`",
            })
        )
    );
}

fn admin_by_id_test(bragi: &mut BragiHandler) {
    let all_20 = bragi.get("/features/admin:fr:77288");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);
    let count = count_types(&types, "city");
    assert_eq!(count, 1);

    assert_eq!(get_values(&all_20, "id"), vec!["admin:fr:77288"]);
}

fn street_by_id_test(bragi: &mut BragiHandler) {
    let all_20 = bragi.get("/features/street:osm:way:161162362");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);

    let count = count_types(&types, "street");
    assert_eq!(count, 1);

    assert_eq!(get_values(&all_20, "id"), vec!["street:osm:way:161162362"]);
}

fn addr_by_id_test(bragi: &mut BragiHandler) {
    let all_20 = bragi.get("/features/addr:2.68385;48.50539");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);
    let count = count_types(&types, "house");
    assert_eq!(count, 1);
    assert_eq!(get_values(&all_20, "id"), vec!["addr:2.68385;48.50539"]);
}

fn stop_by_id_test(bragi: &mut BragiHandler) {
    // search with id
    let response = bragi.get("/features/stop_area:SA:second_station?pt_dataset[]=dataset1");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:second_station");
}

fn stop_area_that_does_not_exists(bragi: &mut BragiHandler) {
    // search with id
    assert_eq!(
        bragi.get_unchecked_json("/features/stop_area:SA:second_station::AA?pt_dataset[]=dataset1"),
        (
            actix_web::http::StatusCode::NOT_FOUND,
            json!({
                "long": "Unable to find object",
                "short": "query error"
            })
        )
    );
}

fn stop_area_invalid_index(bragi: &mut BragiHandler) {
    // if the index does not exists, we get a 404 with "Unable to find object" too
    // it's not trivial to get a better error than a not found object (like a 'not found dataset' error)
    // because the data might just not have been imported yet
    assert_eq!(
        bragi.get_unchecked_json(
            "/features/stop_area:SA:second_station::AA?pt_dataset[]=invalid_dataset"
        ),
        (
            actix_web::http::StatusCode::NOT_FOUND,
            json!({
                "long": "Unable to find object",
                "short": "query error"
            })
        )
    );
}
