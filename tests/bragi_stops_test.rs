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
use super::BragiHandler;
use super::get_value;


pub fn bragi_stops_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // ******************************************
    // we the OSM dataset, three-cities bano dataset and a stop file
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf
    // - bano-three_cities
    // - stops.txt
    // ******************************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    info!("Launching {}", osm2mimir);
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    ::launch_and_assert(bano2mimir,
                        vec!["--input=./tests/fixtures/bano-three_cities.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    let stops2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    info!("Launching {}", stops2mimir);
    ::launch_and_assert(stops2mimir,
                        vec!["--input=./tests/fixtures/stops.txt".into(),
                             "--dataset=dataset1".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    stop_attached_to_admin_test(&bragi);
    stop_no_admin_test(&bragi);
}


fn stop_attached_to_admin_test(bragi: &BragiHandler) {
    // with this query we should find only one response, a stop
    let response = bragi.get("/autocomplete?q=14 juillet");
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
    let response = bragi.get("/autocomplete?q=Far west station");
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
