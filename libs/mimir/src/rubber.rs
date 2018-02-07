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

use super::objects::{Admin, MimirObject};
use chrono;
use hyper;
use hyper::status::StatusCode;
use rs_es::error::EsError;
use rs_es;
use rs_es::EsResponse;
use serde_json;
use rs_es::operations::search::ScanResult;
use serde;
use std;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use super::objects::{AliasOperation, AliasOperations, AliasParameter, Coord, Place};
use rs_es::units::Duration;
use rs_es::units as rs_u;
use rs_es::query::Query;
use rs_es::operations::search::SearchResult;

const SYNONYMS: [&'static str; 17] = [
    "cc,centre commercial",
    "hotel de ville,mairie",
    "gare sncf,gare",
    "chu,chr,hopital",
    "ld,lieu-dit",
    "st,saint",
    "ste,sainte",
    "bvd,bld,bd,boulevard",
    "pt,pont",
    "rle,ruelle",
    "rte,route",
    "vla,villa",
    "grand-champ,grandchamp",
    "fac,faculte,ufr,universite",
    "embarcadere,gare maritime",
    "cpam,securite sociale",
    "anpe,pole emploi",
];

// Rubber is an wrapper around elasticsearch API
pub struct Rubber {
    es_client: rs_es::Client,
    // some operation are not implemented in rs_es, we need to use a raw http client
    http_client: hyper::client::Client,
}

pub struct TypedIndex<T> {
    name: String,
    _type: PhantomData<T>,
}

impl<T> TypedIndex<T> {
    pub fn new(name: String) -> TypedIndex<T> {
        TypedIndex {
            name: name,
            _type: PhantomData,
        }
    }
}

/// return the index associated to the given type and dataset
/// this will be an alias over another real index
pub fn get_main_type_and_dataset_index<T: MimirObject>(dataset: &str) -> String {
    format!("munin_{}_{}", T::doc_type(), dataset)
}

/// return the index associated to the given type
/// this will be an alias over another real index
pub fn get_main_type_index<T: MimirObject>() -> String {
    format!("munin_{}", T::doc_type())
}

pub fn get_date_index_name(base_index_name: &str) -> String {
    format!(
        "{}_{}",
        base_index_name,
        chrono::Utc::now().format("%Y%m%d_%H%M%S_%f")
    )
}

pub fn get_indexes_by_type(a_type: &str) -> String {
    let doc_type = match a_type {
        "public_transport:stop_area" => "stop",
        "city" => "admin",
        "house" => "addr",
        _ => a_type,
    };

    format!("munin_{}", doc_type)
}

pub fn collect(result: SearchResult<serde_json::Value>) -> Result<Vec<Place>, EsError> {
    debug!(
        "{} documents found in {} ms",
        result.hits.total, result.took
    );
    // for the moment rs-es does not handle enum Document,
    // so we need to convert the ES glob to a Place
    Ok(result
        .hits
        .hits
        .into_iter()
        .filter_map(|hit| make_place(hit.doc_type, hit.source))
        .collect())
}

/// takes a ES json blob and build a Place from it
/// it uses the _type field of ES to know which type of the Place enum to fill
pub fn make_place(doc_type: String, value: Option<Box<serde_json::Value>>) -> Option<Place> {
    value.and_then(|v| {
        fn convert<T>(v: serde_json::Value, f: fn(T) -> Place) -> Option<Place>
        where
            for<'de> T: serde::Deserialize<'de>,
        {
            serde_json::from_value::<T>(v)
                .map_err(|err| warn!("Impossible to load ES result: {}", err))
                .ok()
                .map(f)
        }
        match doc_type.as_ref() {
            "addr" => convert(*v, Place::Addr),
            "street" => convert(*v, Place::Street),
            "admin" => convert(*v, Place::Admin),
            "poi" => convert(*v, Place::Poi),
            "stop" => convert(*v, Place::Stop),
            _ => {
                warn!("unknown ES return value, _type field = {}", doc_type);
                None
            }
        }
    })
}

/// Create a `rs_es::Query` that boosts results according to the
/// distance to `coord`.
pub fn build_proximity_with_boost(coord: &Coord, boost: f64) -> Query {
    Query::build_function_score()
        .with_boost(boost)
        .with_function(
            rs_es::query::functions::Function::build_decay(
                "coord",
                rs_u::Location::LatLon(coord.lat(), coord.lon()),
                rs_u::Distance::new(50f64, rs_u::DistanceUnit::Kilometer),
            ).build_gauss(),
        )
        .build()
}

pub fn is_existing_index(client: &mut rs_es::Client, index: &str) -> Result<bool, EsError> {
    if index.is_empty() {
        return Ok(false);
    }
    match client.open_index(&index) {
        //This error indicates that the search index is absent in ElasticSearch.
        Err(EsError::EsError(_)) => Ok(false),
        Err(e) => Err(e),
        Ok(_) => Ok(true),
    }
}

pub fn get_indexes(
    all_data: bool,
    pt_datasets: &[&str],
    types: &[&str],
    client: &mut rs_es::Client,
) -> Result<Vec<String>, EsError> {
    make_indexes_impl(all_data, pt_datasets, types, |index| {
        is_existing_index(client, index)
    })
}

pub fn make_indexes_impl<F: FnMut(&str) -> Result<bool, EsError>>(
    all_data: bool,
    pt_datasets: &[&str],
    types: &[&str],
    mut is_existing_index: F,
) -> Result<Vec<String>, EsError> {
    if all_data {
        return Ok(vec!["munin".to_string()]);
    }

    let mut result: Vec<String> = vec![];
    let mut push = |result: &mut Vec<_>, i: &str| -> Result<(), EsError> {
        if try!(is_existing_index(i)) {
            result.push(i.into());
        }
        Ok(())
    };

    let mut pt_dataset_indexes: Vec<String> = vec![];
    match pt_datasets.len() {
        0 => (),
        1 => for pt_dataset in pt_datasets.iter() {
            try!(push(
                &mut pt_dataset_indexes,
                format!("munin_stop_{}", pt_dataset).as_str(),
            ))
        },
        _ => try!(push(&mut pt_dataset_indexes, "munin_global_stops")),
    };

    match types.len() {
        0 => {
            try!(push(&mut result, &"munin_geo_data".to_string()));
            result.append(&mut pt_dataset_indexes);
        }
        _ => {
            for type_ in types.iter().filter(|t| **t != "public_transport:stop_area") {
                try!(push(&mut result, &get_indexes_by_type(type_)));
            }
            if types.contains(&"public_transport:stop_area") {
                result.append(&mut pt_dataset_indexes);
            }
        }
    }

    Ok(result)
}

impl Rubber {
    // build a rubber with a connection string (http://host:port/)
    pub fn new(cnx: &str) -> Rubber {
        info!("elastic search host {} ", cnx);

        Rubber {
            es_client: rs_es::Client::new(&cnx).unwrap(),
            http_client: hyper::client::Client::new(),
        }
    }

    pub fn get(&self, path: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a get on {}", path);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client.get(&url).send());
        rs_es::do_req(result)
    }
    fn put(&self, path: &str, body: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a put on {} with {}", path, body);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client.put(&url).body(body).send());
        rs_es::do_req(result)
    }
    fn post(&self, path: &str, body: &str) -> Result<hyper::client::response::Response, EsError> {
        // Note: a bit duplicate on rs_es because some ES operations are not implemented
        debug!("doing a post on {} with {}", path, body);
        let url = self.es_client.full_url(path);
        let result = try!(self.http_client.post(&url).body(body).send());
        rs_es::do_req(result)
    }

    pub fn make_index<T: MimirObject>(&self, dataset: &str) -> Result<TypedIndex<T>, String> {
        let index_name = get_date_index_name(&get_main_type_and_dataset_index::<T>(dataset));
        info!("creating index {}", index_name);
        self.create_index(&index_name.to_string())?;
        Ok(TypedIndex::new(index_name))
    }

    pub fn create_index(&self, name: &String) -> Result<(), String> {
        debug!("creating index");
        // Note: in rs_es it can be done with MappingOperation but for the moment I think
        // storing the mapping in json is more convenient
        let analysis = include_str!("../../../json/settings.json");

        let mut analysis_json_value = try!(
            serde_json::from_str::<serde_json::Value>(&analysis).map_err(|err| format!("{}", err))
        );

        let synonyms: Vec<_> = SYNONYMS
            .iter()
            .map(|s| serde_json::Value::String(s.to_string()))
            .collect();

        *analysis_json_value
            .pointer_mut("/settings/analysis/filter/synonym_filter/synonyms")
            .unwrap() = serde_json::Value::Array(synonyms);

        self.put(name, &analysis_json_value.to_string())
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

    pub fn create_template(&self, name: &str, settings: &str) -> Result<(), String> {
        debug!("creating template");
        self.put(&format!("_template/{}", name), settings)
            .map_err(|e| {
                info!("Error while creating template {}", name);
                e.to_string()
            })
            .and_then(|res| {
                if res.status == StatusCode::Ok {
                    Ok(())
                } else {
                    Err(format!("cannot create template: {:?}", res))
                }
            })
    }

    pub fn initialize_templates(&self) -> Result<(), String> {
        self.create_template(
            &"template_addr",
            include_str!("../../../json/addr_settings.json"),
        )?;
        self.create_template(
            &"template_stop",
            include_str!("../../../json/stop_settings.json"),
        )?;
        self.create_template(
            &"template_admin",
            include_str!("../../../json/admin_settings.json"),
        )?;
        self.create_template(
            &"template_street",
            include_str!("../../../json/street_settings.json"),
        )?;
        self.create_template(
            &"template_poi",
            include_str!("../../../json/poi_settings.json"),
        )?;
        Ok(())
    }

    // get all aliases for a doc_type/dataset
    // return a map with each index as key and all their aliases
    pub fn get_all_aliased_index(
        &self,
        base_index: &str,
    ) -> Result<BTreeMap<String, Vec<String>>, String> {
        self.get(&format!("{}*/_aliases", base_index))
            .map_err(|e| e.to_string())
            .and_then(|res| match res.status {
                StatusCode::Ok => {
                    let value: serde_json::Value =
                        try!(res.read_response().map_err(|e| e.to_string()));
                    Ok(value
                        .as_object()
                        .map(|all_aliases| {
                            all_aliases
                                .iter()
                                .filter_map(|(i, a)| {
                                    a.pointer("/aliases").and_then(|a| a.as_object()).map(
                                        |aliases| (i.clone(), aliases.keys().cloned().collect()),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_else(|| {
                            info!("no aliases for {}", base_index);
                            BTreeMap::new()
                        }))
                }
                StatusCode::NotFound => {
                    info!("impossible to find alias {}", base_index);
                    Ok(BTreeMap::new())
                }
                _ => Err(format!("invalid elasticsearch response: {:?}", res)),
            })
    }

    // get the last indexes for this doc_type/dataset
    // Note: to be resilient to ghost ES indexes, we return all indexes for this doc_type/dataset
    // but the new index
    fn get_last_index<T: MimirObject>(
        &self,
        new_index: &TypedIndex<T>,
        dataset: &str,
    ) -> Result<Vec<String>, String> {
        let base_index = get_main_type_and_dataset_index::<T>(dataset);
        // we don't want to remove the newly created index
        Ok(self.get_all_aliased_index(&base_index)?
            .into_iter()
            .map(|(k, _)| k)
            .filter(|i| i.as_str() != new_index.name)
            .collect())
    }

    pub fn get_address(&mut self, coord: &Coord) -> Result<Vec<Place>, EsError> {
        let types = vec!["house".into(), "street".into()];
        let indexes = get_indexes(false, &[], &types, &mut self.es_client)?;
        let distance = rs_u::Distance::new(1000., rs_u::DistanceUnit::Meter);
        let geo_distance =
            Query::build_geo_distance("coord", (coord.lat(), coord.lon()), distance).build();
        let query = Query::build_bool()
            .with_should(build_proximity_with_boost(coord, 1.))
            .with_must(geo_distance)
            .build();

        let result: SearchResult<serde_json::Value> = self.es_client
            .search_query()
            .with_indexes(&indexes
                .iter()
                .map(|index| index.as_str())
                .collect::<Vec<_>>())
            .with_query(&query)
            .with_size(1)
            .send()?;
        collect(result)
    }

    /// publish the index as the new index for this doc_type and this dataset
    /// move the index alias of the doc_type and the dataset to point to this indexes
    /// and remove the old index
    pub fn publish_index<T: MimirObject>(
        &mut self,
        dataset: &str,
        index: TypedIndex<T>,
    ) -> Result<(), String> {
        debug!("publishing index");
        let last_indexes = try!(self.get_last_index(&index, dataset));

        let dataset_index = get_main_type_and_dataset_index::<T>(dataset);
        try!(self.alias(&dataset_index, &vec![index.name.clone()], &last_indexes,));

        let type_index = get_main_type_index::<T>();
        try!(self.alias(&type_index, &vec![dataset_index.clone()], &last_indexes,));

        if T::is_geo_data() {
            try!(self.alias("munin_geo_data", &vec![type_index.to_string()], &vec![],));
            try!(self.alias("munin", &vec!["munin_geo_data".to_string()], &vec![],));
        } else {
            try!(self.alias("munin", &vec![type_index.to_string()], &vec![]));
        }
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
    pub fn alias(&self, alias: &str, add: &[String], remove: &[String]) -> Result<(), String> {
        info!(
            "for {}, adding alias {:?}, removing {:?}",
            alias, add, remove
        );
        let add_operations = add.iter().map(|x| AliasOperation {
            remove: None,
            add: Some(AliasParameter {
                index: x.clone(),
                alias: alias.to_string(),
            }),
        });
        let remove_operations = remove.iter().map(|x| AliasOperation {
            add: None,
            remove: Some(AliasParameter {
                index: x.clone(),
                alias: alias.to_string(),
            }),
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
                    error!(
                        "failed to change aliases for {}, es response: {:?}",
                        alias, res
                    );
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

    pub fn bulk_index<T, I>(
        &mut self,
        index: &TypedIndex<T>,
        iter: I,
    ) -> Result<usize, rs_es::error::EsError>
    where
        T: MimirObject,
        I: Iterator<Item = T>,
    {
        use rs_es::operations::bulk::Action;
        let mut nb = 0;
        let chunk_size = 1000;
        // fold is used for creating the action and optionally set the id of the object
        let mut actions = iter.map(|v| {
            v.es_id()
                .into_iter()
                .fold(Action::index(v), |action, id| action.with_id(id))
        });
        loop {
            let chunk = actions.by_ref().take(chunk_size).collect::<Vec<_>>();

            if chunk.is_empty() {
                break;
            }
            nb += chunk.len();
            try!(
                self.es_client
                    .bulk(&chunk)
                    .with_index(&index.name)
                    .with_doc_type(T::doc_type())
                    .send()
            );

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
    pub fn index<T, I>(&mut self, dataset: &str, iter: I) -> Result<usize, String>
    where
        T: MimirObject,
        I: Iterator<Item = T>,
    {
        // TODO better error handling
        let index = self.make_index(dataset)?;
        let nb_elements = self.bulk_index(&index, iter).map_err(|e| e.to_string())?;
        self.publish_index(dataset, index)?;
        Ok(nb_elements)
    }

    pub fn get_admins_from_dataset(
        &mut self,
        dataset: &str,
    ) -> Result<Vec<Admin>, rs_es::error::EsError> {
        self.get_all_objects_from_index(&get_main_type_and_dataset_index::<Admin>(dataset))
    }

    pub fn get_all_admins(&mut self) -> Result<Vec<Admin>, rs_es::error::EsError> {
        self.get_all_objects_from_index(&get_main_type_index::<Admin>())
    }

    pub fn get_all_objects_from_index<T>(
        &mut self,
        index: &str,
    ) -> Result<Vec<T>, rs_es::error::EsError>
    where
        for<'de> T: MimirObject + serde::de::Deserialize<'de> + std::fmt::Debug,
    {
        let mut result: Vec<T> = vec![];
        let mut scan: ScanResult<T> = self.es_client
            .search_query()
            .with_indexes(&[&index])
            .with_size(1000)
            .with_types(&[&T::doc_type()])
            .scan(&Duration::minutes(1))?;
        loop {
            let page = try!(scan.scroll(&mut self.es_client, &Duration::minutes(1)));
            if page.hits.hits.len() == 0 {
                break;
            }
            result.extend(
                page.hits
                    .hits
                    .into_iter()
                    .filter_map(|hit| hit.source)
                    .map(|ad| *ad),
            );
        }
        try!(scan.close(&mut self.es_client));
        Ok(result)
    }
}

#[test]
pub fn test_valid_url() {
    Rubber::new("http://localhost:9200");
    Rubber::new("localhost:9200");
    Rubber::new("http://bob");
}

#[test]
#[should_panic]
pub fn test_invalid_url_no_port() {
    Rubber::new("localhost");
}

#[test]
fn test_make_indexes_impl() {
    fn ok_index(_index: &str) -> Result<bool, EsError> {
        Ok(true)
    }
    // all_data
    assert_eq!(
        make_indexes_impl(true, &[], &[], ok_index).unwrap(),
        vec!["munin"]
    );

    // no dataset and no types
    assert_eq!(
        make_indexes_impl(false, &[], &[], ok_index).unwrap(),
        vec!["munin_geo_data"]
    );

    // dataset fr + no types
    assert_eq!(
        make_indexes_impl(false, &["fr"], &[], ok_index).unwrap(),
        vec!["munin_geo_data", "munin_stop_fr"]
    );

    // no dataset + types poi, city, street, house and public_transport:stop_area
    // => munin_stop is not included
    assert_eq!(
        make_indexes_impl(
            false,
            &[],
            &[
                "poi",
                "city",
                "street",
                "house",
                "public_transport:stop_area",
            ],
            ok_index,
        ).unwrap(),
        vec!["munin_poi", "munin_admin", "munin_street", "munin_addr"]
    );

    // no dataset fr + type public_transport:stop_area only
    assert_eq!(
        make_indexes_impl(false, &[], &["public_transport:stop_area"], ok_index).unwrap(),
        Vec::<String>::new()
    );

    // dataset fr + types poi, city, street, house and public_transport:stop_area
    assert_eq!(
        make_indexes_impl(
            false,
            &["fr"],
            &[
                "poi",
                "city",
                "street",
                "house",
                "public_transport:stop_area",
            ],
            ok_index,
        ).unwrap(),
        vec![
            "munin_poi",
            "munin_admin",
            "munin_street",
            "munin_addr",
            "munin_stop_fr",
        ]
    );

    // dataset fr types poi, city, street, house without public_transport:stop_area
    //  => munin_stop_fr is not included
    assert_eq!(
        make_indexes_impl(
            false,
            &["fr"],
            &["poi", "city", "street", "house"],
            ok_index,
        ).unwrap(),
        vec!["munin_poi", "munin_admin", "munin_street", "munin_addr"]
    );

    // dataset fr types poi, city, street, house without public_transport:stop_area
    // and the function is_existing_index with a result "false" as non of the index
    // is present in elasticsearch
    assert_eq!(
        make_indexes_impl(
            false,
            &["fr"],
            &["poi", "city", "street", "house"],
            |_index| Ok::<_, EsError>(false),
        ).unwrap(),
        Vec::<String>::new()
    );

    // dataset fr types poi, city, street, house without public_transport:stop_area
    // and the function is_existing_index with an error in the result (Elasticsearch is absent..)
    match make_indexes_impl(
        false,
        &["fr"],
        &["poi", "city", "street", "house"],
        |_index| Err::<bool, _>(EsError::EsError("Elasticsearch".into())),
    ) {
        Err(EsError::EsError(e)) => assert_eq!(e, "Elasticsearch"),
        _ => assert!(false),
    }
}
