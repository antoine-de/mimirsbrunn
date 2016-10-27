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

use hyper;
use hyper::client::Client;
use super::ToJson;

pub fn osm2mimir_bano2mimir_test(es_wrapper: ::ElasticSearchWrapper) {
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/three_cities.osm.pbf".into(),
                             "--import-way".into(),
                             "--import-admin".into(),
                             "--import-poi".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    ::launch_and_assert(bano2mimir,
                        vec!["--input=./tests/fixtures/bano-three_cities.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    // after an import, we should have 4 indexes, and some aliases to this index
    let client = Client::new();
    let res = client.get(&format!("{host}/_aliases", host = es_wrapper.host()))
                    .send()
                    .unwrap();
    assert_eq!(res.status, hyper::Ok);

    let json = res.to_json();

    let raw_indexes = json.as_object().unwrap();
    let first_indexes: Vec<String> = raw_indexes.keys().cloned().collect();

    assert_eq!(first_indexes.len(), 4);
}
