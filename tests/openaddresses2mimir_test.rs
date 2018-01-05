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

use hyper;
use hyper::client::Client;
use mdo::option::{bind, ret};
use super::ToJson;

/// Simple call to a OA load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn oa2mimir_simple_test(es_wrapper: ::ElasticSearchWrapper) {
    let oa2mimir = concat!(env!("OUT_DIR"), "/../../../openaddresses2mimir");
    ::launch_and_assert(
        oa2mimir,
        vec![
            "--input=./tests/fixtures/sample-oa.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let res: Vec<_> = es_wrapper.search_and_filter("72 Otto-Braun-Straße", |_| true).collect();
    assert_eq!(res.len(), 1);

    // after an import, we should have 1 index, and some aliases to this index
    let client = Client::new();
    let res = client
        .get(&format!("{host}/_aliases", host = es_wrapper.host()))
        .send()
        .unwrap();
    assert_eq!(res.status, hyper::Ok);

    let json = res.to_json();
    let raw_indexes = json.as_object().unwrap();
    let first_indexes: Vec<String> = raw_indexes.keys().cloned().collect();
    assert_eq!(first_indexes.len(), 1);

    // our index should be aliased by the master_index + an alias over the document type + dataset
    let aliases = mdo! {
         s =<< raw_indexes.get(first_indexes.first().unwrap());
         s =<< s.as_object();
         s =<< s.get("aliases");
         s =<< s.as_object();
         ret ret(s.keys().cloned().collect())
     }.unwrap_or_else(Vec::new);
    // for the moment 'munin' is hard coded, but hopefully that will change
    assert_eq!(
        aliases,
        vec!["munin", "munin_addr", "munin_addr_fr", "munin_geo_data"]
    );

    // then we import again the open addresse file:
    ::launch_and_assert(
        oa2mimir,
        vec![
            "--input=./tests/fixtures/sample-oa.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    // we should still have only one index (but a different one)
    let res = client
        .get(&format!("{host}/_aliases", host = es_wrapper.host()))
        .send()
        .unwrap();
    assert_eq!(res.status, hyper::Ok);

    let json = res.to_json();
    let raw_indexes = json.as_object().unwrap();
    let final_indexes: Vec<String> = raw_indexes.keys().cloned().collect();

    assert_eq!(final_indexes.len(), 1);
    assert!(final_indexes != first_indexes);

    let aliases = mdo! {
        s =<< raw_indexes.get(final_indexes.first().unwrap());
        s =<< s.as_object();
        s =<< s.get("aliases");
        s =<< s.as_object();
        ret ret(s.keys().cloned().collect())
    }.unwrap_or_else(Vec::new);
    assert_eq!(
        aliases,
        vec!["munin", "munin_addr", "munin_addr_fr", "munin_geo_data"]
    );
}
