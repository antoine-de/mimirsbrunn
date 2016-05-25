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

extern crate curl;
extern crate rs_es;
extern crate serde;
extern crate serde_json;
extern crate regex;

use super::objects::DocType;
use chrono;

use super::objects::{AliasOperations, AliasOperation, AliasParameter};
use serde_json::value::Value;

// Rubber is an wrapper around elasticsearch API
pub struct Rubber {
    client: rs_es::Client,
}

/// return the index associated to the given type and dataset
/// this will be an alias over another real index
fn get_main_index(doc_type: &str, dataset: &str) -> String {
    format!("munin_{}_{}", doc_type, dataset)
}

impl Rubber {
    // build a rubber with a connection string (http://host:port/index)
    pub fn new(cnx: &str) -> Rubber {
        let re = regex::Regex::new(r"(?:https?://)?(?P<host>.+?):(?P<port>\d+)").unwrap();
        let cap = re.captures(cnx).unwrap();
        let host = cap.name("host").unwrap();
        let port = cap.name("port").unwrap().parse::<u32>().unwrap();
        info!("elastic search host {:?} port {:?}", host, port);

        Rubber { client: rs_es::Client::new(&host, port) }
    }

    pub fn make_index(&self, doc_type: &str, dataset: &str) -> Result<String, String> {
        let current_time = chrono::UTC::now().format("%Y%m%d_%H%M%S");
        let index_name = format!("munin_{}_{}_{}", doc_type, dataset, current_time);
        if !self.is_existing_index(&index_name).unwrap() {
            info!("creating index {}", index_name);
            self.create_index(&index_name.to_string()).map(|_| index_name)
        } else {
            Ok(index_name)
        }
    }

    fn create_index(&self, name: &String) -> Result<(), String> {
        debug!("creating index");
        // Note: for the moment I don't see an easy way to do this with rs_es
        let analysis = include_str!("../../../json/settings.json");
        curl::http::handle()
            .put(self.client.full_url(&name), analysis)
            .exec()
            .map_err(|e| {
                info!("Error while creating new index {}", name);
                e.to_string()
            })
            .and_then(|res| {
                if res.get_code() == 200 {
                    Ok(())
                } else {
                    Err(format!("cannot create index: {}",
                                ::std::str::from_utf8(res.get_body()).unwrap()))
                }
            })
    }

    fn get_last_index(&self, doc_type: &str, dataset: &str) -> Result<Vec<String>, String> {
        debug!("get last index: {base_index}/_aliases",
               base_index = get_main_index(doc_type, dataset));
        curl::http::handle()
            .get(self.client.full_url(&format!("{base_index}/_aliases",
                                               base_index = get_main_index(doc_type, dataset))))
            .exec()
            .map_err(|e| e.to_string())
            .and_then(|res| {
                match res.get_code() {
                    200 => {
                        let body = ::std::str::from_utf8(res.get_body()).unwrap();
                        let value: Value = ::serde_json::from_str(body).unwrap();
                        Ok(value.as_object()
                                .and_then(|aliases| Some(aliases.keys().cloned().collect()))
                                .unwrap_or_else(|| {
                                    info!("no previous index to delete for type {} and dataset {}",
                                          doc_type,
                                          dataset);
                                    vec![]
                                }))
                    }
                    404 => {
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
            try!(self.delete_index(&i))
        }
        Ok(())
    }

    fn is_existing_index(&self, name: &String) -> Result<bool, String> {
        curl::http::handle()
            .get(self.client.full_url(&name))
            .exec()
            .map_err(|e| e.to_string())
            .map(|res| res.get_code() == 200)
    }

    /// add a list of new indexes to the alias
    /// remove a list of indexes from the alias
    fn alias(&self, alias: &str, add: &Vec<String>, remove: &Vec<String>) -> Result<(), String> {
        debug!("adding aliases for {}", alias);
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
        let e = curl::http::handle()
                    .post(self.client.full_url("_aliases"), &json)
                    .exec();
        e.map_err(|e| e.to_string())
         .and_then(|res| {
             info!("es response: {}",
                   ::std::str::from_utf8(res.get_body()).unwrap());
             if res.get_code() == 200 {
                 Ok(())
             } else {
                 error!("es response: {}", res);
                 Err("failed".to_string())
             }
         })
    }

    pub fn delete_index(&mut self, index: &String) -> Result<(), String> {
        debug!("deleting index {}", &index);
        let res = self.client
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
                            mut iter: I)
                            -> Result<u32, rs_es::error::EsError>
        where T: serde::Serialize + DocType,
              I: Iterator<Item = T>
    {
        use self::rs_es::operations::bulk::Action;
        let mut chunk = Vec::new();
        let mut nb = 0;
        loop {
            chunk.clear();
            let addr = match iter.next() {
                Some(a) => a,
                None => break,
            };
            chunk.push(Action::index(addr));

            nb += 1;
            for addr in iter.by_ref().take(1000) {
                chunk.push(Action::index(addr));
                nb += 1;
            }
            try!(self.client
                     .bulk(&chunk)
                     .with_index(&index)
                     .with_doc_type(T::doc_type())
                     .send());
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
        let index = self.make_index(doc_type, dataset).unwrap();
        let nb_elements = self.bulk_index(&index, iter).unwrap();
        self.publish_index(doc_type, dataset, index).unwrap();
        Ok(nb_elements)
    }

}
