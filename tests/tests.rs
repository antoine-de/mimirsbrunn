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
#![recursion_limit = "128"]

#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
#[macro_use]
extern crate approx;
#[macro_use]
extern crate assert_float_eq;

mod bano2mimir_test;
mod bragi_bano_test;
mod bragi_filter_types_test;
mod bragi_ntfs_test;
mod bragi_osm_test;
mod bragi_poi_test;
mod bragi_stops_test;
mod bragi_synonyms_test;
mod bragi_three_cities_test;
mod canonical_import_process_test;
mod cosmogony2mimir_test;
mod openaddresses2mimir_test;
mod osm2mimir_bano2mimir_test;
mod osm2mimir_test;
mod rubber_test;
mod stops2mimir_test;

use actix_web::client::ClientResponse;
use docker_wrapper::*;
use failure::{format_err, Error};
use serde_json::value::Value;
use serde_json::Map;
use std::process::Command;

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

        let res = reqwest::Client::new()
            .get(&format!("{}/_refresh", self.host()))
            .send()
            .unwrap();
        assert!(
            res.status() == reqwest::StatusCode::OK,
            "Error ES refresh: {:?}",
            res
        );
    }

    pub fn new(docker_wrapper: &DockerWrapper) -> ElasticSearchWrapper<'_> {
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
        let mut res = self
            .rubber
            .get(&format!("munin/_search?q={}", word))
            .unwrap();
        assert!(res.status() == reqwest::StatusCode::OK);
        res.json().unwrap()
    }

    pub fn search_on_global_stop_index(&self, word: &str) -> serde_json::Value {
        let mut res = self
            .rubber
            .get(&format!("munin_global_stops/_search?q={}", word))
            .unwrap();
        assert!(res.status() == reqwest::StatusCode::OK);
        res.json().unwrap()
    }

    pub fn search_and_filter<'b, F>(
        &self,
        word: &str,
        predicate: F,
    ) -> impl Iterator<Item = mimir::Place> + 'b
    where
        F: 'b + FnMut(&mimir::Place) -> bool,
    {
        self.search_and_filter_on_index(word, predicate, false)
    }

    pub fn search_and_filter_on_global_stop_index<'b, F>(
        &self,
        word: &str,
        predicate: F,
    ) -> impl Iterator<Item = mimir::Place> + 'b
    where
        F: 'b + FnMut(&mimir::Place) -> bool,
    {
        self.search_and_filter_on_index(word, predicate, true)
    }

    fn search_and_filter_on_index<'b, F>(
        &self,
        word: &str,
        predicate: F,
        search_on_global_stops: bool,
    ) -> impl Iterator<Item = mimir::Place> + 'b
    where
        F: 'b + FnMut(&mimir::Place) -> bool,
    {
        use serde_json::map::Entry;

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
        let json = if search_on_global_stops {
            self.search_on_global_stop_index(word)
        } else {
            self.search(word)
        };
        get(json, "hits")
            .and_then(|json| get(json, "hits"))
            .and_then(|hits| match hits {
                Value::Array(v) => Some(v),
                _ => None,
            })
            .unwrap_or_else(Vec::default)
            .into_iter()
            .filter_map(|json| {
                into_object(json).and_then(|obj| {
                    let doc_type = obj
                        .get("_type")
                        .and_then(|doc_type| doc_type.as_str())
                        .map(|doc_type| doc_type.into());

                    doc_type.and_then(|doc_type| {
                        // The real object is contained in the _source section.
                        obj.get("_source").and_then(|src| {
                            bragi::query::make_place(doc_type, Some(Box::new(src.clone())))
                        })
                    })
                })
            })
            .filter(predicate)
    }
}

fn launch_and_assert(
    cmd: &str,
    args: &[std::string::String],
    es_wrapper: &ElasticSearchWrapper<'_>,
) {
    let status = Command::new(cmd).args(args).status().unwrap();
    assert!(status.success(), "`{}` failed {}", cmd, &status);
    es_wrapper.refresh();
}

pub struct BragiHandler {
    app: actix_web::test::TestServer,
}

impl BragiHandler {
    pub fn new(url: String) -> BragiHandler {
        let make_server = move || {
            bragi::server::create_server(bragi::Context::from(&bragi::Args {
                connection_string: url.clone(),
                ..Default::default()
            }))
        };

        BragiHandler {
            app: actix_web::test::TestServer::with_factory(make_server),
        }
    }

    pub fn raw_get(&mut self, q: &str) -> Result<ClientResponse, Error> {
        self.app
            .execute(
                self.app
                    .client(actix_web::http::Method::GET, q)
                    .finish()
                    .map_err(|e| format_err!("invalid query: {}", e))?
                    .send(),
            )
            .map_err(|e| format_err!("impossible to query bragi: {}", e))
    }

    pub fn get(&mut self, q: &str) -> Vec<Map<String, Value>> {
        let r = self.raw_get(q).unwrap();
        assert!(r.status().is_success(), "invalid status: {}", r.status());
        self.get_results(r, Some("/properties/geocoding".to_string()))
    }

    pub fn get_json(&mut self, q: &str) -> Value {
        let r = self.raw_get(q).unwrap();
        assert!(r.status().is_success(), "invalid status: {}", r.status());
        self.to_json(r)
    }

    pub fn get_unchecked_json(&mut self, q: &str) -> (actix_web::http::StatusCode, Value) {
        let r = self.raw_get(q).unwrap();
        (r.status(), self.to_json(r))
    }

    pub fn raw_post_shape(
        &mut self,
        q: &str,
        shape: &'static str,
    ) -> Result<ClientResponse, Error> {
        self.app
            .execute(
                self.app
                    .client(actix_web::http::Method::POST, q)
                    .header(actix_web::http::header::CONTENT_TYPE, "application/json")
                    .body(shape)
                    .map_err(|e| format_err!("invalid query: {}", e))?
                    .send(),
            )
            .map_err(|e| format_err!("impossible to query bragi: {}", e))
    }

    pub fn post_shape(&mut self, q: &str, shape: &'static str) -> Vec<Map<String, Value>> {
        let r = self.raw_post_shape(q, shape).unwrap();
        self.get_results(r, Some("/properties/geocoding".to_string()))
    }

    pub fn to_json(&mut self, r: ClientResponse) -> Value {
        use actix_web::HttpMessage;
        let bytes = self.app.execute(r.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        serde_json::from_str(body).unwrap()
    }

    pub fn get_results(
        &mut self,
        r: ClientResponse,
        pointer: Option<String>,
    ) -> Vec<Map<String, Value>> {
        self.to_json(r)
            .pointer("/features")
            .expect("wrongly formated bragi response")
            .as_array()
            .expect("features must be array")
            .iter()
            .map(|f| {
                if let Some(p) = &pointer {
                    f.pointer(&p)
                        .expect("no field in bragi response")
                        .as_object()
                        .unwrap()
                        .clone()
                } else {
                    f.as_object().unwrap().clone()
                }
            })
            .collect()
    }
}

pub fn get_values<'a>(r: &'a [Map<String, Value>], val: &'a str) -> Vec<&'a str> {
    r.iter().map(|e| get_value(e, val)).collect()
}

pub fn get_value<'a>(e: &'a Map<String, Value>, val: &str) -> &'a str {
    e.get(val).and_then(|l| l.as_str()).unwrap_or_else(|| "")
}

pub fn get_types(r: &[Map<String, Value>]) -> Vec<&str> {
    get_values(r, "type")
}

pub fn filter_by(r: &[Map<String, Value>], key: &str, t: &str) -> Vec<Map<String, Value>> {
    r.iter()
        .filter(|e| e.get(key).and_then(|l| l.as_str()).unwrap_or_else(|| "") == t)
        .cloned()
        .collect()
}

pub fn filter_by_type(r: &[Map<String, Value>], t: &str) -> Vec<Map<String, Value>> {
    filter_by(r, "type", t)
}

pub fn count_types(types: &[&str], value: &str) -> usize {
    types.iter().filter(|&t| *t == value).count()
}

fn get_poi_type_ids(e: &Map<String, Value>) -> Vec<&str> {
    let array = match e.get("poi_types").and_then(|json| json.as_array()) {
        None => return vec![],
        Some(array) => array,
    };
    array
        .iter()
        .filter_map(|v| v.as_object().and_then(|o| o.get("id")))
        .filter_map(|o| o.as_str())
        .collect()
}

fn get_first_index_aliases(indexes: &serde_json::Map<String, Value>) -> Vec<String> {
    indexes
        .get(indexes.keys().next().unwrap())
        .and_then(Value::as_object)
        .and_then(|s| s.get("aliases"))
        .and_then(Value::as_object)
        .map(|s| s.keys().cloned().collect())
        .unwrap_or_else(Vec::new)
}

/// Main test method (regroups all tests)
/// All tests are done sequentially,
/// and use the same docker in order to avoid multiple inits
/// (ES cleanup is handled by `es_wrapper`)
#[test]
fn all_tests() {
    let _guard = mimir::logger_init();
    let docker_wrapper = DockerWrapper::new().unwrap();

    // we call all tests here
    bano2mimir_test::bano2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_test::osm2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    stops2mimir_test::stops2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
    osm2mimir_bano2mimir_test::osm2mimir_bano2mimir_test(ElasticSearchWrapper::new(
        &docker_wrapper,
    ));
    rubber_test::rubber_zero_downtime_test(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_custom_id(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_ghost_index_cleanup(ElasticSearchWrapper::new(&docker_wrapper));
    rubber_test::rubber_empty_bulk(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_bano_test::bragi_bano_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_osm_test::bragi_osm_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_poi_test::test_i18n_poi(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_three_cities_test::bragi_three_cities_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_poi_test::bragi_poi_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_stops_test::bragi_stops_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_ntfs_test::bragi_ntfs_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_filter_types_test::bragi_filter_types_test(ElasticSearchWrapper::new(&docker_wrapper));
    bragi_synonyms_test::bragi_synonyms_test(ElasticSearchWrapper::new(&docker_wrapper));
    openaddresses2mimir_test::oa2mimir_simple_test(ElasticSearchWrapper::new(&docker_wrapper));
    cosmogony2mimir_test::cosmogony2mimir_test(ElasticSearchWrapper::new(&docker_wrapper));
    canonical_import_process_test::canonical_import_process_test(ElasticSearchWrapper::new(
        &docker_wrapper,
    ));
    canonical_import_process_test::bragi_invalid_es_test(ElasticSearchWrapper::new(
        &docker_wrapper,
    ));
}
