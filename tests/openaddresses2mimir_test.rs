// Copyright © 2018, Canal TP and/or its affiliates. All rights reserved.
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

use super::get_first_index_aliases;
use reqwest;
use std::path::Path;

/// Simple call to a OA load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn oa2mimir_simple_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let oa2mimir = Path::new(env!("OUT_DIR"))
        .join("../../../openaddresses2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &oa2mimir,
        &[
            "--input=./tests/fixtures/sample-oa.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let res: Vec<_> = es_wrapper
        .search_and_filter("72 Otto-Braun-Straße", |_| true)
        .collect();
    assert_eq!(res.len(), 1);

    // after an import, we should have 1 index, and some aliases to this index
    let mut res = reqwest::get(&format!("{host}/_aliases", host = es_wrapper.host())).unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::Value = res.json().unwrap();
    let raw_indexes = json.as_object().unwrap();
    let first_indexes: Vec<String> = raw_indexes.keys().cloned().collect();
    assert_eq!(first_indexes.len(), 1);

    // our index should be aliased by the master_index + an alias over the document type + dataset
    let aliases = get_first_index_aliases(raw_indexes);
    // for the moment 'munin' is hard coded, but hopefully that will change
    assert_eq!(
        aliases,
        vec!["munin", "munin_addr", "munin_addr_fr", "munin_geo_data"]
    );

    // then we import again the open addresse file:
    crate::launch_and_assert(
        &oa2mimir,
        &[
            "--input=./tests/fixtures/sample-oa.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    // we should still have only one index (but a different one)
    let mut res = reqwest::get(&format!("{host}/_aliases", host = es_wrapper.host())).unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::Value = res.json().unwrap();
    let raw_indexes = json.as_object().unwrap();
    let final_indexes: Vec<String> = raw_indexes.keys().cloned().collect();

    assert_eq!(final_indexes.len(), 1);
    assert!(final_indexes != first_indexes);

    let aliases = get_first_index_aliases(raw_indexes);
    assert_eq!(
        aliases,
        vec!["munin", "munin_addr", "munin_addr_fr", "munin_geo_data"]
    );

    // we should have imported 10 elements
    // (we should have the one without hash, but not the badly formated line)
    let res: Vec<_> = es_wrapper.search_and_filter("*.*", |_| true).collect();
    assert_eq!(res.len(), 10);

    // We look for 'Fake-City' which should have been filtered since the street name is empty
    let res: Vec<_> = es_wrapper
        .search_and_filter("Fake-City", |_| true)
        .collect();
    assert_eq!(res.len(), 0);
}
