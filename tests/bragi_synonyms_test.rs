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
use super::get_values;
use super::BragiHandler;

pub fn bragi_synonyms_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // ******************************************
    // we the OSM dataset, three-cities bano dataset and a stop file
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf
    // - bano-three_cities
    // - stops.txt
    // ******************************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(
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
    ::launch_and_assert(
        bano2mimir,
        vec![
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let stops2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    ::launch_and_assert(
        stops2mimir,
        vec![
            "--input=./tests/fixtures/stops.txt".into(),
            "--dataset=dataset1".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    synonyms_test(&bragi);
}

fn synonyms_test(bragi: &BragiHandler) {
    // Test that we find Hôtel de Ville
    let response = bragi.get("/autocomplete?q=hotel de ville");
    assert!(get_values(&response, "label")
        .iter()
        .all(|r| r.contains("Hôtel de Ville")));

    // Test we find the same result as above as mairie is synonym of hotel de ville
    let response = bragi.get("/autocomplete?q=mairie");
    assert!(get_values(&response, "label")
        .iter()
        .all(|r| r.contains("Hôtel de Ville")));
}
