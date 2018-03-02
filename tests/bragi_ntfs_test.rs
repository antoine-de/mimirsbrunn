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
extern crate iron;
extern crate iron_test;
extern crate serde_json;
use super::BragiHandler;
use super::get_value;

pub fn bragi_ntfs_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    let ntfs2mimir = concat!(env!("OUT_DIR"), "/../../../ntfs2mimir");
    ::launch_and_assert(
        ntfs2mimir,
        vec![
            "--input=./tests/fixtures/ntfs/".into(),
            "--dataset=dataset1".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    gare_de_lyon(&bragi);

    let ntfs2mimir = concat!(env!("OUT_DIR"), "/../../../ntfs2mimir");
    ::launch_and_assert(
        ntfs2mimir,
        vec![
            "--input=./tests/fixtures/ntfs2/".into(),
            "--dataset=dataset2".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    gare_de_lyon_with_two_datasets(&bragi);
}

fn gare_de_lyon(bragi: &BragiHandler) {
    // with this query we should find only one response, a stop
    let response = bragi.get("/autocomplete?q=gare de lyon&_all_data=true");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "Gare de Lyon");
    assert_eq!(get_value(stop, "name"), "Gare de Lyon");
    assert_eq!(get_value(stop, "id"), "stop_area:GDL");
    assert_eq!(get_value(stop, "timezone"), "Europe/Paris");
    assert_eq!(
        stop.get("physical_modes").unwrap(),
        &json!([
            {"id": "physical_mode:Bus", "name": "Bus"},
            {"id": "physical_mode:Metro", "name": "Metro"},
            {"id": "physical_mode:RapidTransit", "name": "Rapid Transit"}
        ])
    );
    assert_eq!(
        stop.get("commercial_modes").unwrap(),
        &json!([
            {"id": "commercial_mode:Bus", "name": "Bus"},
            {"id": "commercial_mode:Metro", "name": "Metro"},
            {"id": "commercial_mode:RER", "name": "Réseau Express Régional (RER)"}
        ])
    );
    assert_eq!(
        stop.get("codes").unwrap(),
        &json!([
            {"name": "navitia1", "value": "424242"},
            {"name": "source", "value": "stop_area:GDL"},
        ])
    );
    assert_eq!(
        stop.get("properties").unwrap(),
        &json!([
            {"key": "awesome_system", "value": "id:4242"},
        ])
    );
}

fn gare_de_lyon_with_two_datasets(bragi: &BragiHandler) {
    // with this query we should find only one response, a stop
    let response =
        bragi.get("/autocomplete?q=gare de lyon&pt_dataset=dataset1&pt_dataset=dataset2");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();

    assert_eq!(get_value(stop, "type"), "public_transport:stop_area");
    assert_eq!(get_value(stop, "label"), "Gare de Lyon");
    assert_eq!(get_value(stop, "name"), "Gare de Lyon");
    assert_eq!(get_value(stop, "id"), "stop_area:GDL");
    assert_eq!(get_value(stop, "timezone"), "Europe/Paris");

    assert_eq!(
        stop.get("physical_modes").unwrap(),
        &json!([
            {"id": "physical_mode:Bus", "name": "Bus"},
            {"id": "physical_mode:Metro", "name": "Metro"},
            {"id": "physical_mode:Metro", "name": "Underground"}, // From dataset2
            {"id": "physical_mode:RapidTransit", "name": "Rapid Transit"}
        ])
    );
    assert_eq!(
        stop.get("commercial_modes").unwrap(),
        &json!([
            {"id": "commercial_mode:Bus", "name": "Bus"},
            {"id": "commercial_mode:Metro", "name": "Metro"},
            {"id": "commercial_mode:Metro", "name": "Underground"}, // From dataset2
            {"id": "commercial_mode:RER", "name": "Réseau Express Régional (RER)"}
        ])
    );
    assert_eq!(
        stop.get("codes").unwrap(),
        &json!([
            {"name": "navitia1", "value": "424242"},
            {"name": "navitia2", "value": "434343"}, // From dataset2
            {"name": "source", "value": "stop_area:GDL"},
        ])
    );
    assert_eq!(
        stop.get("properties").unwrap(),
        &json!([
            {"key": "awesome_system", "value": "id:4242"},
            {"key": "super_awesome_system", "value": "id:4343"}, // From dataset2
        ])
    );
}
