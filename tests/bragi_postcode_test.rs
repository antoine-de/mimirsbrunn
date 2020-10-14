// Copyright © 2020, Canal TP and/or its affiliates. All rights reserved.
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

pub fn bragi_postcode_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let mut bragi = BragiHandler::new(es_wrapper.host());
    let out_dir = Path::new(env!("OUT_DIR"));

    // Import regular OSM dataset, together with an address with an housenumber of same value as
    // Melun's postcode.
    let osm2mimir = Path::new(env!("OUT_DIR"))
        .join("../../../osm2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &osm2mimir,
        &[
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--config-dir=./config".into(),
            "--import-admin".into(),
            "--import-way".into(),
            "--level=8".into(),
            "--level=7".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let oa2mimir = out_dir
        .join("../../../openaddresses2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &oa2mimir,
        &[
            "--input=./tests/fixtures/oa_postcode.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    // When a postcode is searched, we don't expect any address with a matching house number.
    let response = bragi.get("/autocomplete?q=77000");
    assert!(!response.is_empty());
    assert!(response
        .iter()
        .all(|res| get_value(res, "housenumber") != "77000"));

    // The address can still be found when more info is given
    let response = bragi.get("/autocomplete?q=77000 Southwest");
    assert!(response
        .iter()
        .any(|res| get_value(res, "street") == "7th Line Southwest"));

    // When searching for a number only, we should still be able to find road names
    let response = bragi.get("/autocomplete?q=18");
    assert!(response
        .iter()
        .any(|res| get_value(res, "street") == "Rue des 18 Arpents"));
}
