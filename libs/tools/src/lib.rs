use actix_web::client::ClientResponse;
use docker_wrapper::*;
use failure::{format_err, Error};
use serde_json::value::Value;
use serde_json::Map;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;

pub struct ElasticSearchWrapper<'a> {
    pub docker_wrapper: &'a DockerWrapper,
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
    //    should be == 0 if indexes are ok (no refresh needed)
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

    /// count the number of documents in the index
    /// If you want to count eg the number of POI, you would call
    /// es_wrapper.count("_type:POI")
    pub fn count(&self, word: &str) -> u64 {
        info!("counting documents with munin/_count?q={}", word);
        let mut res = self
            .rubber
            .get(&format!("munin/_count?q={}", word))
            .unwrap();
        assert!(res.status() == reqwest::StatusCode::OK);
        let json: serde_json::Value = res.json().unwrap();
        json["count"].as_u64().unwrap()
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

pub struct BragiHandler {
    pub app: actix_web::test::TestServer,
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
