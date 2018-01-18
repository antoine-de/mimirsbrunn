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

extern crate bragi;
extern crate iron_test;
extern crate serde_json;
use super::BragiHandler;
use super::get_values;
use super::get_value;
use super::get_types;
use super::count_types;

pub fn bragi_osm_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // *********************************
    // We load the OSM dataset (including ways)
    // *********************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(
        osm2mimir,
        vec![
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-way".into(),
            "--level=8".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    zip_code_test(&bragi);
    zip_code_street_test(&bragi);
    zip_code_admin_test(&bragi);
}

fn zip_code_test(bragi: &BragiHandler) {
    let all_20 = bragi.get("/autocomplete?q=77000");
    assert_eq!(all_20.len(), 10);
    for postcodes in get_values(&all_20, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    assert!(
        get_values(&all_20, "postcode")
            .iter()
            .any(|r| *r == "77000;77003;77008;CP77001")
    );

    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 7);

    let count = count_types(&types, "city");
    assert_eq!(count, 3);

    let count = count_types(&types, "house");
    assert_eq!(count, 0);
}

fn zip_code_street_test(bragi: &BragiHandler) {
    let res = bragi.get("/autocomplete?q=77000 Lotissement le Clos de Givry");
    assert_eq!(res.len(), 1);
    let le_clos = &res[0];
    assert_eq!(le_clos["postcode"], "77000");
    assert_eq!(
        le_clos["label"],
        "Lotissement le Clos de Givry (Livry-sur-Seine)"
    );
    assert_eq!(le_clos["name"], "Lotissement le Clos de Givry");
    assert_eq!(le_clos["street"], "Lotissement le Clos de Givry");

    let boundary = le_clos["administrative_regions"].pointer("/0/boundary");
    assert_eq!(boundary, None);

    assert_eq!(le_clos["type"], "street");
    assert_eq!(le_clos["citycode"], "77255");
}

fn zip_code_admin_test(bragi: &BragiHandler) {
    let all_20 = bragi.get("/autocomplete?q=77000 Vaux-le-Pénil");
    assert_eq!(all_20.len(), 4);
    assert!(
        get_values(&all_20, "postcode")
            .iter()
            .all(|r| *r == "77000",)
    );
    let types = get_types(&all_20);
    let count = count_types(&types, "street");
    assert_eq!(count, 3);

    let count = count_types(&types, "city");
    assert_eq!(count, 1);
    let first_city = all_20.iter().find(|e| get_value(e, "type") == "city");
    assert_eq!(get_value(first_city.unwrap(), "citycode"), "77487");

    let count = count_types(&types, "house");
    assert_eq!(count, 0);
}
