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

use mimir::Members;

/// Simple call to a BANO load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn osm2mimir_sample_test(es_wrapper: ::ElasticSearchWrapper) {
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
    // Test: Import of Admin
    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Livry-sur-Seine", |_| true)
        .collect();
    assert_eq!(res.len(), 5);

    let has_boundary = |res: &Vec<mimir::Place>, is_admin: bool| {
        res.iter()
            .filter(|place| place.is_admin() == is_admin)
            .flat_map(|a| a.admins())
            .all(|a| a.boundary.clone().map_or(false, |b| !b.0.is_empty()))
    };
    // Admins have boundaries
    assert!(has_boundary(&res, true));

    // Others places than Admin don't
    assert!(!has_boundary(&res, false));

    assert!(res.iter().any(|r| r.is_admin()));

    // Test: search for "Rue des Près"
    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Rue des Près", |_| true)
        .collect();
    assert!(res.len() != 0);
    assert!(res[0].is_street());
    // The first hit should be "Rue des Près"
    assert!(res[0].label() == "Rue des Près (Livry-sur-Seine)");

    // And there should be only ONE "Rue des Près"
    assert_eq!(
        res.iter()
            .filter(|place| {
                place.is_street() && place.label() == "Rue des Près (Livry-sur-Seine)"
            })
            .count(),
        1
    );

    // Test: Search for "Rue du Four à Chaux" in "Livry-sur-Seine"
    let place_filter = |place: &mimir::Place| {
        place.is_street() && place.label() == "Rue du Four à Chaux (Livry-sur-Seine)" &&
            place
                .admins()
                .first()
                .map(|admin| admin.label() == "Livry-sur-Seine (77000)")
                .unwrap_or(false)
    };
    // As we merge all ways with same name and of the same admin(level=city_level)
    // Here we have only one way
    let nb = es_wrapper
        .search_and_filter("label:Rue du Four à Chaux (Livry-sur-Seine)", place_filter)
        .count();
    assert_eq!(nb, 1);

    // Test: Streets having the same label in different cities
    let place_filter = |place: &mimir::Place| {
        place.is_street() && place.label() == "Rue du Port (Melun)" &&
            place
                .admins()
                .first()
                .map(|admin| admin.label() == "Melun (77000-CP77001)")
                .unwrap_or(false)
    };
    let nb = es_wrapper
        .search_and_filter("label:Rue du Port (Melun)", place_filter)
        .count();
    assert_eq!(nb, 1);

    // Test: search Pois by label
    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Le-Mée-sur-Seine Courtilleraies", |_| true)
        .collect();
    assert!(res.len() != 0);

    let poi_type_post_office = "poi_type:amenity:post_office";
    assert!(res.iter().any(|r| {
        r.poi().map_or(
            false,
            |poi| poi.poi_type.id == poi_type_post_office,
        )
    }));

    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Melun Rp", |_| true)
        .collect();
    assert!(res.len() != 0);
    assert!(res.iter().any(|r| {
        r.poi().map_or(
            false,
            |poi| poi.poi_type.id == poi_type_post_office,
        )
    }));
}
