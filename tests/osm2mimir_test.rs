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
use serde_json::value::Value;

/// Simple call to a BANO load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn osm2mimir_sample_test(es_wrapper: ::ElasticSearchWrapper) {
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(osm2mimir,
                      vec!["--input=./tests/fixtures/three_cities.osm.pbf".into(),
                       "--import-way".into(),
                       "--import-admin".into(),
                       "--level=8".into(),
                       format!("--connection-string={}", es_wrapper.host())],
                       &es_wrapper);

    // Test: Import of Admin
    let city = es_wrapper.search("label:Livry-sur-Seine");
    let nb_hits = city.pointer("/hits/total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(nb_hits, 1);
    let city_type = city.pointer("/hits/hits/0/_type").and_then(|v| v.as_string()).unwrap_or("");
    assert_eq!(city_type, "admin");
    
    // Test: search for "Rue des Près"
    let search = es_wrapper.search("label:Rue des Près");
    // The first hit should be "Rue des Près"
    let street_label = search.pointer("/hits/hits/0/_source/label").and_then(|v| v.as_string()).unwrap_or("");
    assert_eq!(street_label, "Rue des Près");
    // And there should be only ONE "Rue des Près"
    let street_filter = |street: &Value| {
    	         	let label = street.pointer("/_source/label").and_then(|n| n.as_string()).unwrap_or("");
    	            label == "Rue des Près"
    			};
    let nb = es_wrapper.search_and_filter(r#"label:Rue des Près"#, street_filter).count();
    assert_eq!(nb, 1);
  
    // Test: Search for "Rue du Four à Chaux" in "Livry-sur-Seine"
    let street_filter = |street: &Value| {
    	            let label = street.pointer("/_source/label").and_then(|n| n.as_string()).unwrap_or("");
    	            let admin_label = street.pointer("/_source/administrative_regions/0/label").and_then(|n| n.as_string()).unwrap_or("");            
    	            label == "Rue du Four à Chaux" && admin_label == "Livry-sur-Seine"
    			};
    let nb = es_wrapper.search_and_filter("label:Rue du Four à Chaux", street_filter).count();
    assert_eq!(nb, 6);
    
    //Test: Streets having the same label in different cities
    let street_filter = |street: &Value| {
                        let label = street.pointer("/_source/label").and_then(|n| n.as_string()).unwrap_or("");
                        let admin_label = street.pointer("/_source/administrative_regions/0/label").and_then(|n| n.as_string()).unwrap_or("");
                        label == r#"Rue du Port"# && admin_label == r#"Melun"#
    			};
    let nb = es_wrapper.search_and_filter(r#"label:Rue du Port"#, street_filter).count();
    assert_eq!(nb, 1);
}
