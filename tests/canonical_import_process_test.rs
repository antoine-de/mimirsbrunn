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

extern crate iron;
extern crate mimir;
extern crate serde_json;
use super::count_types;
use super::get_poi_type_ids;
use super::get_types;
use super::get_value;
use super::get_values;
use super::BragiHandler;

/// Test the whole mimirsbrunn pipeline with all the import binary
/// and test thourgh bragi in the end
///
/// First we import cosmogony,
/// then openaddress (or bano),
/// then osm (without any admins)
pub fn canonical_import_process_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));
    ::launch_and_assert(
        concat!(env!("OUT_DIR"), "/../../../cosmogony2mimir"),
        vec![
            "--input=./tests/fixtures/cosmogony.json".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    ::launch_and_assert(
        concat!(env!("OUT_DIR"), "/../../../bano2mimir"),
        vec![
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    ::launch_and_assert(
        concat!(env!("OUT_DIR"), "/../../../osm2mimir"),
        vec![
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-way".into(),
            "--import-poi".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    melun_test(&bragi);
}

fn melun_test(bragi: &BragiHandler) {
    let all_melun = bragi.get("/autocomplete?q=Melun");
    let types = get_types(&all_melun);
    let count = count_types(&types, "city");
    assert_eq!(count, 1);

    // and the city should be the first result
    let melun = all_melun.first().unwrap();
    assert_eq!(melun["name"], "Melun");
    // the fact that we have this label proves that this admins comes
    // from cosmogony as we cannot have this label through osm alone
    assert_eq!(
        melun["label"],
        "Melun (77000-CP77001), Fausse Seine-et-Marne, France hexagonale"
    );
    assert_eq!(melun["postcode"], "77000;77003;77008;CP77001");
    assert_eq!(melun["id"], "admin:osm:relation:80071");
    assert_eq!(melun["type"], "city");
    assert_eq!(melun["city"], serde_json::Value::Null);

    // we should also find other object that are in the city
    // (in the data there is at least one poi, and one street that matches)
    let cityhall = all_melun
        .iter()
        .find(|e| get_value(e, "name") == "Hôtel de Ville")
        .unwrap();
    // the poi should be geocoded and be linked to melun and it's administrative hierarchy
    assert_eq!(cityhall["city"], "Melun");
    assert_eq!(cityhall["label"], "Hôtel de Ville (Melun)");
    assert_eq!(cityhall["postcode"], "77000");
    assert_eq!(cityhall["citycode"], "77288");
    assert_eq!(get_value(cityhall, "type"), "poi");
    assert_eq!(get_poi_type_ids(cityhall), &["poi_type:amenity:townhall"]);
    assert_eq!(cityhall["city"], "Melun");

    let cityhall_admins = cityhall["administrative_regions"]
        .as_array()
        .expect("admins must be array");

    assert_eq!(cityhall_admins.len(), 3);
    assert_eq!(cityhall_admins[0]["id"], "admin:osm:relation:80071");
    assert_eq!(cityhall_admins[0]["insee"], "77288");
    assert_eq!(
        cityhall_admins[0]["label"],
        "Melun (77000-CP77001), Fausse Seine-et-Marne, France hexagonale"
    );
    assert_eq!(cityhall_admins[0]["name"], "Melun");
    assert_eq!(cityhall_admins[0]["zone_type"], "city");

    assert_eq!(cityhall_admins[1]["id"], "admin:osm:relation:424253843");
    assert_eq!(cityhall_admins[1]["insee"], "77");
    assert_eq!(
        cityhall_admins[1]["label"],
        "Fausse Seine-et-Marne, France hexagonale"
    );
    assert_eq!(cityhall_admins[1]["name"], "Fausse Seine-et-Marne");
    assert_eq!(cityhall_admins[1]["zone_type"], "state_district");

    assert_eq!(cityhall_admins[2]["id"], "admin:osm:relation:424256272");
    assert_eq!(cityhall_admins[2]["label"], "France hexagonale");
    assert_eq!(cityhall_admins[2]["name"], "France hexagonale");
    assert_eq!(cityhall_admins[2]["zone_type"], "country");

    // the poi should have been associated to an address
    let poi_addr = cityhall["address"].as_object().unwrap();

    assert_eq!(poi_addr["label"], "2 Rue de la Reine Blanche (Melun)");
    assert_eq!(poi_addr["housenumber"], "2");
    assert_eq!(poi_addr["street"], "Rue de la Reine Blanche");
    assert_eq!(poi_addr["postcode"], "77288");
    assert_eq!(poi_addr["city"], "Melun");
}

pub fn bragi_invalid_es_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("http://invalid_es_url/munin"));

    // the status does not check the ES connexion, so for the status all is good
    let resp = bragi.raw_get("/status").unwrap();
    assert_eq!(resp.status, Some(iron::status::Status::Ok));

    // the autocomplete gives a 503
    let resp = bragi.raw_get("/autocomplete?q=toto").unwrap();
    assert_eq!(resp.status, Some(iron::status::Status::ServiceUnavailable));
}
