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

use super::get_value;
use super::BragiHandler;
use std::path::Path;

pub fn bragi_stops_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let mut bragi = BragiHandler::new(es_wrapper.host());
    let out_dir = Path::new(env!("OUT_DIR"));

    // ******************************************
    // we import the OSM dataset, three-cities bano dataset and 2 stop files
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf
    // - bano-three_cities
    // - stops.txt
    // - stops_dataset2.txt
    // ******************************************
    let osm2mimir = out_dir.join("../../../osm2mimir").display().to_string();
    crate::launch_and_assert(
        &osm2mimir,
        &[
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-admin".into(),
            "--level=8".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let bano2mimir = out_dir.join("../../../bano2mimir").display().to_string();
    crate::launch_and_assert(
        &bano2mimir,
        &[
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let stops2mimir = out_dir.join("../../../stops2mimir").display().to_string();
    crate::launch_and_assert(
        &stops2mimir,
        &[
            "--input=./tests/fixtures/stops.txt".into(),
            "--dataset=dataset1".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    stop_attached_to_admin_test(&mut bragi);
    stop_no_admin_test(&mut bragi);

    let stops2mimir = out_dir.join("../../../stops2mimir").display().to_string();
    crate::launch_and_assert(
        &stops2mimir,
        &[
            "--input=./tests/fixtures/stops_dataset2.txt".into(),
            "--dataset=dataset2".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    stop_filtered_by_dataset_test(&mut bragi);
    autocomplete_stop_filtered_by_dataset_transcoverage_test(&mut bragi);
    features_stop_filtered_by_dataset_transcoverage_test(&mut bragi);
    stop_all_data_test(&mut bragi);
    stop_order_by_weight_test(&mut bragi);
    distance_test(&mut bragi);
}

fn stop_attached_to_admin_test(bragi: &mut BragiHandler) {
    // with this query we should find only one response, a stop
    let response = bragi.get("/autocomplete?q=14 juillet&_all_data=true");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "14 Juillet (Vaux-le-Pénil)");
    assert_eq!(get_value(stop, "name"), "14 Juillet");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:second_station");
    assert_eq!(get_value(stop, "citycode"), "77487");
    assert_eq!(get_value(stop, "postcode"), "77000");

    // this stop area is in the boundary of the admin 'Vaux-le-Pénil',
    // it should have been associated to it
    assert_eq!(get_value(stop, "city"), "Vaux-le-Pénil");
    let admins = stop
        .get("administrative_regions")
        .and_then(|a| a.as_array());
    assert_eq!(admins.map(|a| a.len()).unwrap_or(0), 1);
}

fn stop_no_admin_test(bragi: &mut BragiHandler) {
    // we query another stop, but this one is outside the range of an admin,
    // we should get the stop, but with no admin attached to it
    let response = bragi.get("/autocomplete?q=Far west station&_all_data=true");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "Far west station");
    assert_eq!(get_value(stop, "name"), "Far west station");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:station_no_city");
    assert_eq!(get_value(stop, "city"), "");
    let admins = stop
        .get("administrative_regions")
        .and_then(|a| a.as_array());
    assert_eq!(admins.map(|a| a.len()).unwrap_or(0), 0);
}

fn stop_filtered_by_dataset_test(bragi: &mut BragiHandler) {
    // Search stops on all aliases
    let response = bragi.get("/autocomplete?q=14 juillet&_all_data=true");
    assert_eq!(response.len(), 2);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:second_station");

    let stop = response.last().unwrap();
    assert_eq!(
        get_value(stop, "id"),
        "stop_area:SA:second_station:dataset2"
    );

    // filter by dataset1
    let response = bragi.get("/autocomplete?q=14 juillet&pt_dataset[]=dataset1");

    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:second_station");
    // filter by dataset2
    let response = bragi.get("/autocomplete?q=14 juillet&pt_dataset[]=dataset2");

    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(
        get_value(stop, "id"),
        "stop_area:SA:second_station:dataset2"
    );
}

fn autocomplete_stop_filtered_by_dataset_transcoverage_test(bragi: &mut BragiHandler) {
    //autocomplete endpoint tests
    //Search without dataset
    let response = bragi.get("/autocomplete?q=All known stop");
    assert_eq!(response.len(), 0);

    //Search on all_data (not munin_global_stops)
    let response = bragi.get("/autocomplete?q=All known stop&_all_data=true");
    assert_eq!(response.len(), 2);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    let mut names = vec![get_value(stop, "name")];

    let stop = response.last().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    names.push(get_value(stop, "name"));

    assert!(names.contains(&"All known stop"));
    assert!(names.contains(&"All known stop, but different name"));

    //classic filter by the dataset1
    let response = bragi.get("/autocomplete?q=All known stop&pt_dataset[]=dataset1");
    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(
        get_value(stop, "name"),
        "All known stop, but different name"
    );

    //classic filter by the dataset2
    let response = bragi.get("/autocomplete?q=All known stop&pt_dataset[]=dataset2");

    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(get_value(stop, "name"), "All known stop");

    //filter by multiple datasets (1 matching)
    let response =
        bragi.get("/autocomplete?q=All known stop&pt_dataset[]=dataset2&pt_dataset[]=bobito");
    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(
        get_value(stop, "name"),
        "All known stop, but different name"
    ); //name should be the first binarized

    //filter by multiple datasets (all matching)
    let response =
        bragi.get("/autocomplete?q=All known stop&pt_dataset[]=dataset2&pt_dataset[]=dataset1");
    assert_eq!(response.len(), 1);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(
        get_value(stop, "name"),
        "All known stop, but different name"
    ); //name should be the first binarized

    //filter by multiple datasets (none matching)
    let response =
        bragi.get("/autocomplete?q=All known stop&pt_dataset[]=bobette&pt_dataset[]=bobito");
    assert_eq!(response.len(), 0);
}

fn features_stop_filtered_by_dataset_transcoverage_test(bragi: &mut BragiHandler) {
    //no pt_dataset: no chocolate
    let response = bragi
        .raw_get("/features/stop_area:SA:known_by_all_dataset")
        .unwrap();
    assert_eq!(response.status(), actix_web::http::StatusCode::NOT_FOUND);

    //wrong pt_dataset
    let response = bragi
        .raw_get("/features/stop_area:SA:known_by_all_dataset?pt_dataset[]=bobette")
        .unwrap();
    assert_eq!(response.status(), actix_web::http::StatusCode::NOT_FOUND);

    //wrong pt_datasets
    let response = bragi
        .raw_get(
            "/features/stop_area:SA:known_by_all_dataset?pt_dataset[]=bobette&pt_dataset[]=bobito",
        )
        .unwrap();
    assert_eq!(response.status(), actix_web::http::StatusCode::NOT_FOUND);

    //one matching dataset, we hit the global one
    let response = bragi.get(
        "/features/stop_area:SA:known_by_all_dataset?pt_dataset[]=dataset2&pt_dataset[]=bobito",
    );
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(
        get_value(stop, "name"),
        "All known stop, but different name"
    );

    //one dataset, we hit it (not the global one)
    let response = bragi.get("/features/stop_area:SA:known_by_all_dataset?pt_dataset[]=dataset2");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(get_value(stop, "name"), "All known stop");

    //two matching pt_datasets, hitting the global index ()
    let response = bragi.get(
        "/features/stop_area:SA:known_by_all_dataset?pt_dataset[]=dataset1&pt_dataset[]=dataset2",
    );
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    assert_eq!(
        get_value(stop, "name"),
        "All known stop, but different name"
    );

    //all_data: hitting all the pt indexes
    let response = bragi.get("/features/stop_area:SA:known_by_all_dataset?_all_data=true");
    assert_eq!(response.len(), 2);

    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    let mut names = vec![get_value(stop, "name")];

    let stop = response.last().unwrap();
    assert_eq!(get_value(stop, "id"), "stop_area:SA:known_by_all_dataset");
    names.push(get_value(stop, "name"));

    assert!(names.contains(&"All known stop"));
    assert!(names.contains(&"All known stop, but different name"));
}

fn stop_all_data_test(bragi: &mut BragiHandler) {
    // search without _all_data, default value : _all_data = false
    let response = bragi.get("/autocomplete?q=14 juillet");
    assert_eq!(response.len(), 0);

    // search wiht _all_data = false
    let response = bragi.get("/autocomplete?q=14 juillet&_all_data=false");
    assert_eq!(response.len(), 0);

    // search wiht _all_data = true
    let response = bragi.get("/autocomplete?q=14 juillet&_all_data=true");
    assert_eq!(response.len(), 2);
}

fn stop_order_by_weight_test(bragi: &mut BragiHandler) {
    // The StopAreas are sorted by weight. stop_area:SA:weight_3_station having weight 3
    // will be the first element in the result where as stop_area:SA:weight_1_station will
    //always be second.
    let response = bragi.get("/autocomplete?q=weight&_all_data=true");
    assert_eq!(response.len(), 2);

    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "weight three");
    assert_eq!(get_value(stop, "name"), "weight three");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:weight_3_station");

    let stop = response.last().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "weight one");
    assert_eq!(get_value(stop, "name"), "weight one");
    assert_eq!(get_value(stop, "id"), "stop_area:SA:weight_1_station");
}

fn distance_test(bragi: &mut BragiHandler) {
    // This test highlight distance computing.
    // if {lat,lon} params are added in the request, we compute the distance
    // between input coord and autocomplete objects coords.
    // In the test, the input coord is 100 meters away from autocomplete features coords.

    // with input coord
    {
        let response = bragi
            .raw_get("/autocomplete?q=14 juillet&_all_data=true&lat=48.526578&lon=2.679347")
            .unwrap();
        let features = bragi.get_results(response, None);
        assert_eq!(features.len(), 2);

        let feature_first = features.first().unwrap();
        assert!(feature_first.contains_key("distance"));
        assert_eq!(feature_first["distance"], 100);

        let feature_second = features.last().unwrap();
        assert!(feature_second.contains_key("distance"));
        assert_eq!(feature_second["distance"], 100);
    }

    // without input coord
    {
        let response = bragi
            .raw_get("/autocomplete?q=14 juillet&_all_data=true")
            .unwrap();
        let features = bragi.get_results(response, None);
        assert_eq!(features.len(), 2);

        let feature_first = features.first().unwrap();
        assert!(!feature_first.contains_key("distance"));

        let feature_second = features.last().unwrap();
        assert!(!feature_second.contains_key("distance"));
    }
}
