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

use std::process::Command;
extern crate serde_json;
use serde_json::value::Value;

/// Simple call to a BANO load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn osm2mimir_sample_test(es_wrapper: ::ElasticSearchWrapper) {
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    let status = Command::new(osm2mimir)
                     .args(&["--input=./tests/fixtures/rues_trois_communes.osm.pbf".into(),
                             "--import-way".into(),
                             "--import-admin".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())])
                     .status()
                     .unwrap();
    assert!(status.success(), "`bano2mimir` failed {}", &status);
    es_wrapper.refresh();

    // Test: Import of Admin
    let city = es_wrapper.search("name:Livry-sur-Seine");
    let nb_hits = city.lookup("hits.total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(nb_hits, 1);
    let city_type = city.pointer("/hits/hits/0/_type").and_then(|v| v.as_string()).unwrap_or("");
    assert_eq!(city_type, "admin");
    
    // Test: search for "Rue des Près"
    let search = es_wrapper.search(r#"name:Rue des Près"#);
    // The first hit should be "Rue des Près"
    let rue_name = search.pointer("/hits/hits/0/_source/name").and_then(|v| v.as_string()).unwrap_or("");
    assert_eq!(rue_name, r#"Rue des Près"#);
    // And there should be only ONE "Rue des Près"
    let street_filter = |street: &Value| {
    	         	let name = street.pointer("/_source/name").and_then(|n| n.as_string()).unwrap_or("");
    	            name == r#"Rue des Près"#
    			};
    let nb = es_wrapper.search_and_filter(r#"name:Rue des Près"#, street_filter).count();
    assert_eq!(nb, 1);
  
    // Test: Search for "Rue du Four à Chaux" in "Livry-sur-Seine"
    let street_filter = |street: &Value| {
    	            let name = street.pointer("/_source/name").and_then(|n| n.as_string()).unwrap_or("");
    	            let admin_name = street.pointer("/_source/administrative_regions/0/name").and_then(|n| n.as_string()).unwrap_or("");            
    	            name == r#"Rue du Four à Chaux"# && admin_name == r#"Livry-sur-Seine"#
    			};
    let nb = es_wrapper.search_and_filter(r#"name:Rue du Four à Chaux"#, street_filter).count();
    assert_eq!(nb, 6);
    
    //Test: Streets having the same name in different cities
    let street_filter = |street: &Value| {
                        let name = street.pointer("/_source/name").and_then(|n| n.as_string()).unwrap_or("");
                        let admin_name = street.pointer("/_source/administrative_regions/0/name").and_then(|n| n.as_string()).unwrap_or("");
                        name == r#"Rue du Port"# && admin_name == r#"Melun"#
    			};
    let nb = es_wrapper.search_and_filter(r#"name:Rue du Port"#, street_filter).count();
    assert_eq!(nb, 1);
}
