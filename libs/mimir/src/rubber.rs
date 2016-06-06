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


use super::objects::{DocType, EsId, Admin};
use chrono;
use regex;
use hyper;
use hyper::status::StatusCode;
use rs_es::error::EsError;
use rs_es;
use rs_es::EsResponse;
use serde;
use serde_json;
use rs_es::operations::search::ScanResult;

use super::objects::{AliasOperations, AliasOperation, AliasParameter};
use rs_es::units::{Duration};
use std::collections::BTreeMap;

// Rubber is an wrapper around elasticsearch API
pub struct Rubber {
    es_client: rs_es::Client,
    // some operation are not implemented in rs_es, we need to use a raw http client
    http_client: hyper::client::Client,
}

/// return the index associated to the given type and dataset
/// this will be an alias over another real index
fn get_main_index(doc_type: &str, dataset: &str) -> String {
    format!("munin_{}_{}", doc_type, dataset)
}

impl Rubber {
    // build a rubber with a connection string (http://host:port/)
    pub fn new(cnx: &str) -> Rubber {
        let re = regex::Regex::new(r"(?:https?://)?(?P<host>.+?):(?P<port>\d+)").unwrap();
        let cap = re.captures(cnx).unwrap();
        let host = cap.name("host").unwrap();
        let port = cap.name("port").unwrap().parse::<u32>().unwrap();
        info!("elastic search host {:?} port {:?}", host, port);

        Rubber {
            es_client: rs_es::Client::new(&host, port),
            http_client: hyper::client::Client::new(),
        }
    }

    pub fn get(&self, path: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a get on {}", path);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client
                              .get(&url)
                              .send());
        rs_es::do_req(result)
    }
    fn put(&self, path: &str, body: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a put on {} with {}", path, body);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client
                              .put(&url)
                              .body(body)
                              .send());
        rs_es::do_req(result)
    }
    fn post(&self, path: &str, body: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a post on {} with {}", path, body);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client
                              .post(&url)
                              .body(body)
                              .send());
        rs_es::do_req(result)
    }

    pub fn make_index(&self, doc_type: &str, dataset: &str) -> Result<String, String> {
        let current_time = chrono::UTC::now().format("%Y%m%d_%H%M%S_%f");
        let index_name = format!("munin_{}_{}_{}", doc_type, dataset, current_time);
        info!("creating index {}", index_name);
        self.create_index(&index_name.to_string()).map(|_| index_name)
    }

    fn create_index(&self, name: &String) -> Result<(), String> {
        debug!("creating index");
        // Note: in rs_es it can be done with MappingOperation but for the moment I think
        // storing the mapping in json is more convenient
        let analysis = include_str!("../../../json/settings.json");
        self.put(name, &analysis)
            .map_err(|e| {
                info!("Error while creating new index {}", name);
                e.to_string()
            })
            .and_then(|res| {
                if res.status == StatusCode::Ok {
                    Ok(())
                } else {
                    Err(format!("cannot create index: {:?}", res))
                }
            })
    }

    fn get_last_index(&self, doc_type: &str, dataset: &str) -> Result<Vec<String>, String> {
        debug!("get last index: {base_index}/_aliases",
               base_index = get_main_index(doc_type, dataset));
        self.get(&format!("{base_index}/_aliases",
                          base_index = get_main_index(doc_type, dataset)))
            .map_err(|e| e.to_string())
            .and_then(|res| {
                match res.status {
                    StatusCode::Ok => {
                        let value: serde_json::Value = try!(res.read_response()
                                                               .map_err(|e| e.to_string()));
                        Ok(value.as_object()
                                .and_then(|aliases| Some(aliases.keys().cloned().collect()))
                                .unwrap_or_else(|| {
                                    info!("no previous index to delete for type {} and dataset {}",
                                          doc_type,
                                          dataset);
                                    vec![]
                                }))
                    }
                    StatusCode::NotFound => {
                        info!("impossible to find alias {}, no last index to remove",
                              get_main_index(doc_type, dataset));
                        Ok(vec![])
                    }
                    _ => Err(format!("invalid elasticsearch response: {:?}", res)),
                }

            })
    }

    /// publish the index as the new index for this doc_type and this dataset
    /// move the index alias of the doc_type and the dataset to point to this indexes
    /// and remove the old index
    pub fn publish_index(&mut self,
                         doc_type: &str,
                         dataset: &str,
                         index: String)
                         -> Result<(), String> {
        debug!("publishing index");
        let last_indexes = try!(self.get_last_index(doc_type, dataset));
        let main_index = get_main_index(doc_type, dataset);
        try!(self.alias(&main_index, &vec![index.clone()], &last_indexes));
        try!(self.alias("munin", &vec![main_index.to_string()], &vec![]));
        for i in last_indexes {
            try!(self.delete_index(&i));
        }
        Ok(())
    }

    pub fn is_existing_index(&self, name: &String) -> Result<bool, String> {
        self.get(&name)
            .map_err(|e| e.to_string())
            .map(|res| res.status == StatusCode::Ok)
    }

    /// add a list of new indexes to the alias
    /// remove a list of indexes from the alias
    fn alias(&self, alias: &str, add: &Vec<String>, remove: &Vec<String>) -> Result<(), String> {
        info!("for {}, adding alias {:?}, removing {:?}",
              alias,
              add,
              remove);
        let add_operations = add.iter().map(|x| {
            AliasOperation {
                remove: None,
                add: Some(AliasParameter {
                    index: x.clone(),
                    alias: alias.to_string(),
                }),
            }
        });
        let remove_operations = remove.iter().map(|x| {
            AliasOperation {
                add: None,
                remove: Some(AliasParameter {
                    index: x.clone(),
                    alias: alias.to_string(),
                }),
            }
        });
        let operations = AliasOperations {
            actions: add_operations.chain(remove_operations).collect(),
        };
        let json = serde_json::to_string(&operations).unwrap();
        self.post("_aliases", &json)
            .map_err(|e| e.to_string())
            .and_then(|res| {
                if res.status == StatusCode::Ok {
                    Ok(())
                } else {
                    error!("failed to change aliases for {}, es response: {:?}",
                           alias,
                           res);
                    Err(format!("failed to post aliases for {}: {:?}", alias, res).to_string())
                }
            })
    }

    pub fn delete_index(&mut self, index: &String) -> Result<(), String> {
        debug!("deleting index {}", &index);
        let res = self.es_client
                      .delete_index(&index)
                      .map(|res| res.acknowledged)
                      .unwrap_or(false);
        if !res {
            Err(format!("Error deleting index {}", &index).into())
        } else {
            Ok(())
        }
    }

    pub fn bulk_index<T, I>(&mut self,
                            index: &String,
                            iter: I)
                            -> Result<usize, rs_es::error::EsError>
        where T: serde::Serialize + DocType + EsId,
              I: Iterator<Item = T>
    {
        use rs_es::operations::bulk::Action;
        let mut chunk = Vec::new();
        let mut nb = 0;
        let chunk_size = 1000;
        //fold is used for creating the action and optionally set the id of the object
        let mut actions = iter.map(|v| v.es_id()
                                        .into_iter()
                                        .fold(Action::index(v), |action, id| action.with_id(id)));
        loop {
            let chunk = actions.by_ref().take(chunk_size).collect::<Vec<_>>();
            nb += chunk.len();
            try!(self.es_client
                     .bulk(&chunk)
                     .with_index(&index)
                     .with_doc_type(T::doc_type())
                     .send());

            if chunk.len() < chunk_size {
                break;
            }
        }

        Ok(nb)
    }

    /// add all the element of 'iter' into elasticsearch
    ///
    /// To have zero downtime:
    /// first all the elements are added in a temporary index and when all has been indexed
    /// the index is published and the old index is removed
    pub fn index<T, I>(&mut self, doc_type: &str, dataset: &str, iter: I) -> Result<u32, String>
        where T: serde::Serialize + DocType,
              I: Iterator<Item = T>
    {
        // TODO better error handling
        let index = try!(self.make_index(doc_type, dataset));
        let nb_elements = try!(self.bulk_index(&index, iter).map_err(|e| e.to_string()));
        try!(self.publish_index(doc_type, dataset, index));
        Ok(nb_elements)
    }

	fn make_admin(&self, value: Option<Box<serde_json::Value>>) -> Option<Admin> {
	    value.and_then(|v| {
	    		serde_json::from_value::<Admin>(*v).ok().and_then(|o| Some(o))
	    })
	}

    pub fn get_admins(&mut self) -> Result<BTreeMap<String, Admin>, rs_es::error::EsError> {
        debug!("Get Admins.");
        let mut result: BTreeMap<String, Admin> = BTreeMap::new();
        let scroll = Duration::minutes(1);
        let mut scan: ScanResult<serde_json::Value> =
        match self.es_client
                  .search_query()
                  .with_size(10000)
                  .with_types(&[&"admin"])
                  .scan(&scroll) {
        	Ok(scan) => scan,
        	Err(e) => {
        		info!("Scan error: {:?}", e);
        		return Err(e);
        	}
        };
        loop {
        	let page = match scan.scroll(&mut self.es_client, &scroll) {
        		Ok(page) => page,
        		Err(e) => {
        			info!("scroll error: {:?}", e);
        			try!(scan.close(&mut self.es_client));
        			return Err(e);
        		}
        	};
        	if page.hits.hits.len() == 0 {
        		break;
        	}
        	let tmp: Vec<Admin> = page.hits
        	                          .hits
        	                          .into_iter()
        	                          .filter_map(|hit| self.make_admin(hit.source))
        	                          .collect();
        	for ad in tmp {
        		result.insert(ad.id.to_string(), ad);
        	}
        }
        try!(scan.close(&mut self.es_client));
        Ok(result)
	}
}

#[test]
pub fn test_valid_url() {
    Rubber::new("http://localhost:9200");
    Rubber::new("localhost:9200");
}

#[test]
#[should_panic]
pub fn test_invalid_url() {
    Rubber::new("http://bob");
}

#[test]
#[should_panic]
pub fn test_invalid_url_no_port() {
    Rubber::new("localhost");
}
