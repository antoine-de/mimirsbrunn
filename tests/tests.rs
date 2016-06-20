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

extern crate mimir;
extern crate docker_wrapper;
extern crate hyper;
extern crate rs_es;
extern crate serde_json;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mdo;

use docker_wrapper::*;

mod bano2mimir_test;
mod osm2mimir_test;
mod osm2mimir_bano2mimir_test;
mod rubber_test;
mod bragi_test;
use serde_json::value::Value;
use hyper::client::response::Response;

trait ToJson {
    fn to_json(self) -> Value;
}
impl ToJson for Response {
    fn to_json(self) -> Value {
        match serde_json::from_reader(self) {
            Ok(v) => v,
            Err(e) => {
                panic!("could not get json value from response: {:?}", e);
            }
        }
    }
}


pub struct ElasticSearchWrapper<'a> {
    docker_wrapper: &'a DockerWrapper,
    pub rubber: mimir::rubber::Rubber,
}

impl<'a> ElasticSearchWrapper<'a> {
    pub fn host(&self) -> String {
        self.docker_wrapper.host()
    }

    pub fn init(&mut self) {
        self.rubber.delete_index(&"_all".to_string()).unwrap();
    }

    //    A way to watch if indexes are built might be curl http://localhost:9200/_stats
    //    then _all/total/segments/index_writer_memory_in_bytes( or version_map_memory_in_bytes)
    // 	  should be == 0 if indexes are ok (no refresh needed)
    pub fn refresh(&self) {
        info!("Refreshing ES indexes");

        let res = hyper::client::Client::new()
                      .get(&format!("{}/_refresh", self.host()))
                      .send()
                      .unwrap();
        assert!(res.status == hyper::Ok, "Error ES refresh: {:?}", res);
    }

    pub fn new(docker_wrapper: &DockerWrapper) -> ElasticSearchWrapper {
        let mut es_wrapper = ElasticSearchWrapper {
            docker_wrapper: docker_wrapper,
            rubber: mimir::rubber::Rubber::new(&docker_wrapper.host()),
        };
        es_wrapper.init();
        es_wrapper
    }

    /// simple search on an index
    /// assert that the result is OK and transform it to a json Value
    pub fn search(&self, word: &str) -> serde_json::Value {
        let res = self.rubber.get(&format!("munin/_search?q={}", word)).unwrap();
        assert!(res.status == hyper::Ok);
        res.to_json()
    }
    
    pub fn search_and_filter<'b, F>(&self, word: &str, predicate: F)
    	-> Box<Iterator<Item=serde_json::value::Value> + 'b>
    	where F: 'b + FnMut(&serde_json::value::Value) -> bool
    {
        use serde_json::value::Value;
        use std::collections::btree_map;
    	fn into_object(json: Value) -> Option<btree_map::BTreeMap<String, Value>> {
    	    match json {
    	        Value::Object(o) => Some(o),
    	        _ => None
    	    }
    	}
    	fn get(json: Value, key: &str) -> Option<Value> {
    	    into_object(json).and_then(|mut json| {
    	        match json.entry(key.into()) {
    	        	btree_map::Entry::Occupied(o) => Some(o.remove()),
    	        	_ => None
    	    	}
    	    })
    	}
    	let json = self.search(word);
    	get(json, "hits")
    		.and_then(|json| get(json, "hits"))
            .and_then(|hits| {
                match hits {
                    Value::Array(v) =>
                    	Some(Box::new(v.into_iter().filter(predicate)) as Box<Iterator<Item = Value>>),
                    _ => None
                }
            })
            .unwrap_or(Box::new(None.into_iter()) as Box<Iterator<Item = Value>>)
    }
}

/// Main test method (regroups all tests)
/// All tests are done sequentially,
/// and use the same docker in order to avoid multiple inits
/// (ES cleanup is handled by es_wrapper)
#[test]
fn all_tests() {
    mimir::logger_init().unwrap();
    let docker_wrapper = DockerWrapper::new().unwrap();

    // we call all tests here
    bano2mimir_test::bano2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_test::osm2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_bano2mimir_test::osm2mimir_bano2mimir_test(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_zero_downtime_test(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_custom_id(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_test::bragi_tests(ElasticSearchWrapper::new(&docker_wrapper));
}
