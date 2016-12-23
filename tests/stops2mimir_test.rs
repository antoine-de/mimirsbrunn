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

extern crate serde_json;
extern crate mimir;

/// Simple call to a BANO load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn stops2mimir_sample_test(es_wrapper: ::ElasticSearchWrapper) {
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/stops.txt".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);
    // Test: Import of Admin
    let res: Vec<_> = es_wrapper.search_and_filter("*", |_| true).collect();
    assert_eq!(res.len(), 3);
    assert!(res.iter().all(|r| r.is_stop()));

    // Test: search for stop area not in ES base
    let res: Vec<_> = es_wrapper.search_and_filter("label:unknown", |_| true).collect();
    assert!(res.len() == 0);

    // Test: search for "République"
    let res: Vec<_> = es_wrapper.search_and_filter("label:République", |_| true).collect();
    assert!(res.len() == 1);
    assert_eq!(res[0].label(), "République");
    assert!(res[0].admins().is_empty());
}
