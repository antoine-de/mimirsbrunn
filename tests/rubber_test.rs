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

use serde_json::value::{to_value, Value};
use mimir::{Street, Admin, Coord, CoordWrapper};
use mimir::rubber::Rubber;
use std;
use std::cell::Cell;

fn check_has_elt(es: &::ElasticSearchWrapper, fun: Box<Fn(&Value)>) {
    let search = es.search("*:*"); // we get all documents in the base
    // we should have our elt
    assert_eq!(search.lookup("hits.total").and_then(|v| v.as_u64()).unwrap_or(0),
               1);
    let es_elt = search.lookup("hits.hits")
                       .and_then(|h| h.as_array())
                       .and_then(|v| v.first())
                       .unwrap();
    fun(es_elt);
}

fn check_has_bob(es: &::ElasticSearchWrapper) {
    let check_is_bob = |es_elt: &Value| {
        assert_eq!(es_elt.find("_type").and_then(|t| t.as_string()).unwrap(),
                   "street");
        let es_bob = es_elt.find("_source").unwrap();
        assert_eq!(es_bob.find("id"), Some(&to_value("bob")));
        assert_eq!(es_bob.find("street_name"), Some(&to_value("bob's street")));
        assert_eq!(es_bob.find("label"), Some(&to_value("bob's name")));
        assert_eq!(es_bob.find("weight"), Some(&Value::U64(42)));
    };
    check_has_elt(es, Box::new(check_is_bob));
}

/// check the zero downtime update
/// first load a batch a data, and then upload a second one
/// during the second batch we should be able to query Elasticsearch and find the first batch
pub fn rubber_zero_downtime_test(mut es: ::ElasticSearchWrapper) {
    info!("running rubber_zero_downtime_test");
    let doc_type = "street";
    let dataset = "my_dataset";

    let bob = Street {
        id: "bob".to_string(),
        street_name: "bob's street".to_string(),
        label: "bob's name".to_string(),
        administrative_regions: vec![],
        weight: 42u32,
        zip_codes: vec![],
        coord: Coord {
            lat: 0.,
            lon: 0.,
        },
    };

    // we index our bob
    let result = es.rubber.index(doc_type, dataset, std::iter::once(bob));

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // we have indexed 1 element

    es.refresh(); // we need to refresh the index to be sure to get the elt;

    check_has_bob(&es);

    let bobette = Street {
        id: "bobette".to_string(),
        street_name: "bobette's street".to_string(),
        label: "bobette's name".to_string(),
        administrative_regions: vec![],
        weight: 24u32,
        zip_codes: vec![],
        coord: Coord {
            lat: 48.5110722f64,
            lon: 2.68326290f64,
        },
    };

    info!("inserting bobette");
    let mut rubber = Rubber::new(&es.docker_wrapper.host());
    // while yielding the new street, we want to check that we are still
    let checker_iter = std::iter::once(bobette).inspect(|_| {
        es.refresh(); // we send a refresh to be sure to be up to date
        check_has_bob(&es);
    });
    let result = rubber.index(doc_type, dataset, checker_iter);
    assert!(result.is_ok(),
            "impossible to index bobette, res: {:?}",
            result);
    assert_eq!(result.unwrap(), 1); // we still have only indexed 1 element

    es.refresh(); // we send another refresh

    // then we should have our bobette
    let check_is_bobette = |es_elt: &Value| {
        assert_eq!(es_elt.find("_type").and_then(|t| t.as_string()).unwrap(),
                   "street");
        let es_bob = es_elt.find("_source").unwrap();
        assert_eq!(es_bob.find("id"), Some(&to_value("bobette")));
        assert_eq!(es_bob.find("street_name"),
                   Some(&to_value("bobette's street")));
        assert_eq!(es_bob.find("label"), Some(&to_value("bobette's name")));
        assert_eq!(es_bob.find("weight"), Some(&Value::U64(24)));
        
        let es_coord = es_bob.find("coord").unwrap();
        assert_eq!(es_coord.find("lat"), Some(&Value::F64(48.5110722)));
        assert_eq!(es_coord.find("lon"), Some(&Value::F64(2.68326290)));
        
    };
    check_has_elt(&es, Box::new(check_is_bobette));
}

pub fn rubber_custom_id(mut es: ::ElasticSearchWrapper) {
    info!("running rubber_custom_id");
    let doc_type = "admin";
    let dataset = "my_dataset";

    let admin = Admin {
        id: "admin:bob".to_string(),
        insee: "insee:dummy".to_string(),
        level: 8,
        label: "my admin".to_string(),
        zip_codes: vec!["zip_code".to_string()],
        weight: Cell::new(42),
        coord: CoordWrapper::new(48.5110722f64, 2.68326290f64),
        boundary: None,
    };

    // we index our admin
    let result = es.rubber.index(doc_type, dataset, std::iter::once(admin));

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // we have indexed 1 element

    es.refresh(); // we need to refresh the index to be sure to get the elt;

    let check_admin = |es_elt: &Value| {
        assert_eq!(es_elt.find("_type").and_then(|t| t.as_string()).unwrap(),
                   "admin");
        let es_source = es_elt.find("_source").unwrap();
        assert_eq!(es_elt.find("_id"), es_source.find("id"));
        assert_eq!(es_elt.find("_id"), Some(&to_value("admin:bob")));
        assert_eq!(es_source.find("insee"), Some(&to_value("insee:dummy")));
        
        let es_coord = es_source.find("coord").unwrap();
        assert_eq!(es_coord.find("lat"), Some(&Value::F64(48.5110722)));
        assert_eq!(es_coord.find("lon"), Some(&Value::F64(2.68326290)));
    };
    check_has_elt(&es, Box::new(check_admin));
}
