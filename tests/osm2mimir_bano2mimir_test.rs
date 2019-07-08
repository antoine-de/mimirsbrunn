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

use reqwest;
use std::path::Path;

pub fn osm2mimir_bano2mimir_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let out_dir = Path::new(env!("OUT_DIR"));

    let osm2mimir = out_dir.join("../../../osm2mimir").display().to_string();
    crate::launch_and_assert(
        &osm2mimir,
        &[
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-way".into(),
            "--import-admin".into(),
            "--import-poi".into(),
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

    // after an import, we should have 4 indexes, and some aliases to this index
    let mut res = reqwest::get(&format!("{host}/_aliases", host = es_wrapper.host())).unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::Value = res.json().unwrap();

    let raw_indexes = json.as_object().unwrap();
    let first_indexes: Vec<String> = raw_indexes.keys().cloned().collect();

    assert_eq!(first_indexes.len(), 4);
}
