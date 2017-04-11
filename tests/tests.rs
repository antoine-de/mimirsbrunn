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
extern crate rustless;
extern crate iron;
extern crate iron_test;
extern crate mime;
#[macro_use]
extern crate serde_json;
extern crate bragi;
#[macro_use]
extern crate log;
#[macro_use]
extern crate mdo;

mod bano2mimir_test;
mod osm2mimir_test;
mod stops2mimir_test;
mod osm2mimir_bano2mimir_test;
mod rubber_test;
mod bragi_bano_test;
mod bragi_osm_test;
mod bragi_three_cities_test;
mod bragi_poi_test;
mod bragi_stops_test;
mod bragi_filter_types_test;
mod bragi_synonyms_test;

use docker_wrapper::*;
use serde_json::Map;
use serde_json::value::Value;
use hyper::client::response::Response;
use std::process::Command;

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

    pub fn search_and_filter<'b, F>(&self,
                                    word: &str,
                                    predicate: F)
                                    -> Box<Iterator<Item = mimir::Place> + 'b>
        where F: 'b + FnMut(&mimir::Place) -> bool
    {
        use serde_json::value::Value;
        use serde_json::map::{Map, Entry};
        fn into_object(json: Value) -> Option<Map<String, Value>> {
            match json {
                Value::Object(o) => Some(o),
                _ => None,
            }
        }
        fn get(json: Value, key: &str) -> Option<Value> {
            into_object(json).and_then(|mut json| match json.entry(key.to_string()) {
                Entry::Occupied(o) => Some(o.remove()),
                _ => None,
            })
        }
        let json = self.search(word);
        get(json, "hits")
            .and_then(|json| get(json, "hits"))
            .and_then(|hits| {
                match hits {
                    Value::Array(v) => {
                        Some(Box::new(v.into_iter()
                            .filter_map(|json| {
                                into_object(json).and_then(|obj| {
                                    let doc_type = obj.get("_type")
                                        .and_then(|doc_type| doc_type.as_str())
                                        .map(|doc_type| doc_type.into());

                                    doc_type.and_then(|doc_type| {
                                        // The real object is contained in the _source section.
                                        obj.get("_source").and_then(|src| {
                                            bragi::query::make_place(doc_type,
                                                                     Some(Box::new(src.clone())))
                                        })
                                    })
                                })
                            })
                            .filter(predicate)) as
                             Box<Iterator<Item = mimir::Place>>)
                    }
                    _ => None,
                }
            })
            .unwrap_or(Box::new(None.into_iter()) as Box<Iterator<Item = mimir::Place>>)
    }
}

fn launch_and_assert(cmd: &'static str,
                     args: Vec<std::string::String>,
                     es_wrapper: &ElasticSearchWrapper) {
    let status = Command::new(cmd).args(&args).status().unwrap();
    assert!(status.success(), "`{}` failed {}", cmd, &status);
    es_wrapper.refresh();
}

pub struct BragiHandler {
    app: rustless::Application,
}

impl BragiHandler {
    pub fn new(url: String) -> BragiHandler {
        let api = bragi::api::ApiEndPoint { es_cnx_string: url }.root();
        BragiHandler { app: rustless::Application::new(api) }
    }

    pub fn raw_get(&self, q: &str) -> iron::IronResult<iron::Response> {
        iron_test::request::get(&format!("http://localhost:3000{}", q),
                                iron::Headers::new(),
                                &self.app)
    }

    pub fn get(&self, q: &str) -> Vec<Map<String, Value>> {
        get_results(self.raw_get(q).unwrap())
    }

    pub fn raw_post_shape(&self, q: &str, shape: &str) -> iron::IronResult<iron::Response> {
        let mut header = iron::Headers::new();
        let mime: mime::Mime = "application/json".parse().unwrap();
        header.set(iron::headers::ContentType(mime));

        iron_test::request::post(&format!("http://localhost:3000{}", q),
                                 header,
                                 shape,
                                 &self.app)
    }

    pub fn post_shape(&self, q: &str, shape: &str) -> Vec<Map<String, Value>> {
        get_results(self.raw_post_shape(q, shape).unwrap())
    }
}

pub fn to_json(r: iron::Response) -> Value {
    let s = iron_test::response::extract_body_to_string(r);
    serde_json::from_str(&s).unwrap()
}

pub fn get_results(r: iron::Response) -> Vec<Map<String, Value>> {
    to_json(r)
        .pointer("/features")
        .expect("wrongly formated bragi response")
        .as_array()
        .expect("features must be array")
        .iter()
        .map(|f| {
            f.pointer("/properties/geocoding")
                .expect("no geocoding object in bragi response")
                .as_object()
                .unwrap()
                .clone()
        })
        .collect()
}

pub fn get_values<'a>(r: &'a [Map<String, Value>], val: &'a str) -> Vec<&'a str> {
    r.iter().map(|e| get_value(e, val)).collect()
}

pub fn get_value<'a>(e: &'a Map<String, Value>, val: &'a str) -> &'a str {
    e.get(val).and_then(|l| l.as_str()).unwrap_or("")
}

pub fn get_types(r: &[Map<String, Value>]) -> Vec<&str> {
    r.iter().map(|e| e.get("type").and_then(|l| l.as_str()).unwrap_or("")).collect()
}

pub fn filter_by_type<'a>(r: &'a [Map<String, Value>], t: &'a str) -> Vec<Map<String, Value>> {
    r.iter()
        .filter(|e| e.get("type").and_then(|l| l.as_str()).unwrap_or("") == t)
        .cloned()
        .collect()
}

pub fn count_types(types: &[&str], value: &str) -> usize {
    types.iter().filter(|&t| *t == value).count()
}

/// Main test method (regroups all tests)
/// All tests are done sequentially,
/// and use the same docker in order to avoid multiple inits
/// (ES cleanup is handled by `es_wrapper`)
#[test]
fn all_tests() {
    mimir::logger_init().unwrap();
    let docker_wrapper = DockerWrapper::new().unwrap();

    // we call all tests here
    bano2mimir_test::bano2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_test::osm2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    stops2mimir_test::stops2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_bano2mimir_test::osm2mimir_bano2mimir_test(
        ElasticSearchWrapper::new(&docker_wrapper)
    );
    rubber_test::rubber_zero_downtime_test(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_custom_id(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_ghost_index_cleanup(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_bano_test::bragi_bano_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_osm_test::bragi_osm_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_three_cities_test::bragi_three_cities_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_poi_test::bragi_poi_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_stops_test::bragi_stops_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_filter_types_test::bragi_filter_types_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_synonyms_test::bragi_synonyms_test(ElasticSearchWrapper::new(&docker_wrapper));
}
