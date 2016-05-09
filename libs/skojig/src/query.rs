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
use std::str;
use std::vec;
use curl;
use super::model;
use rustc_serialize::json::Json;
use rs_es;
use rs_es::query::Query as rs_q;
use serde_json;
//use mimir;
pub type CurlResult = Result<curl::http::Response, curl::ErrCode>;

// TODO: this enum should be moved to rubbser so that we don't create cycled dependencies
// pub enum ResType {
//     Admin(mimir::Admin),
// }


fn query(q: &String) -> Result<curl::http::Response, curl::ErrCode> {
    use rustc_serialize::json::Json::String;

    let subQuery = rs_q::build_bool()
                      .with_should(vec![rs_q::build_term("_type","addr").with_boost(1000)
                                  .build(),
                              rs_q::build_match("name.prefix", q.to_string())
                                  .with_boost(100)
                                  .build(),
                              rs_q::build_function_score()
                                  .with_boost_mode(rs_es::query::compound::BoostMode::Multiply)
                                  .with_boost(30)
                                  .with_query(rs_q::build_match_all().build())
				  .with_function(rs_es::query::functions::Function::build_field_value_factor("")
				      .with_factor(1)
				      .with_modifier(rs_es::query::functions::Modifier::Log1p)
				      .build())
                                  .build()])
                      .build();
    let filter = rs_q::build_bool()
                     .with_should(vec![rs_q::build_bool()
                                           .with_must_not(rs_q::build_exists("house_number")
                                                              .build())
                                           .build(),
                                       rs_q::build_match("house_number", q.to_string()).build()])
                     .with_must(vec![rs_q::build_match("name.prefix", q.to_string()).with_minimum_should_match(rs_es::query::MinimumShouldMatch::from(100f64)).build()])
                     .build();

    let final_query = rs_q::build_bool()
                          .with_must(vec![subQuery])
                          .with_filter(filter)
                          .build();

    let mut client = rs_es::Client::new("localhost", 9200);
    
    /* TODO: Query is constructed, only have to send the query and the typed response
    let result = client.search_query()
                       .with_indexes(&["munin"])
                       .with_query(&final_query)
                       .send()
                       .unwrap();

    */ 
    
    let query = format!(include_str!("../../../json/query_exact.json"),
                        query = String(q.to_string()));
    let resp = try!(curl::http::handle()
                    .post("http://localhost:9200/munin/_search?pretty", &query)
                    .exec());
    let body = Json::from_str(str::from_utf8(resp.get_body()).unwrap()).unwrap();
    if body["hits"]["total"].as_u64().unwrap() > 0 { return Ok(resp); }
    let query = format!(include_str!("../../../json/query.json"), query=String(q.to_string()));
    let resp = curl::http::handle()
        .post("http://localhost:9200/munin/_search?pretty", &query)
        .exec();
    resp
}

fn query_location(q: &String, coord: &model::Coord) -> Result<curl::http::Response, curl::ErrCode> {
    panic!("todo!");
    /*
    use rustc_serialize::json::Json::String;
    let query = format!(include_str!("../../../json/query_exact_location.json"),
                        query=String(q.to_string()),
                        lon=coord.lon,
                        lat=coord.lat);
    let resp = try!(curl::http::handle()
                    .post("http://localhost:9200/munin/_search?pretty", &query)
                    .exec());
    let body = Json::from_str(str::from_utf8(resp.get_body()).unwrap()).unwrap();
    if body["hits"]["total"].as_u64().unwrap() > 0 { return Ok(resp); }
    let query = format!(include_str!("../../../json/query_location.json"),
                        query=String(q.to_string()),
                        lon=coord.lon,
                        lat=coord.lat);
    let resp = curl::http::handle()
        .post("http://localhost:9200/munin/_search?pretty", &query)
        .exec();
    resp*/
}
/*
fn get_val<T>(json: &Json, path: &[&str]) -> T {
    json.find_path(path).map(|j| j.clone()).unwrap_or("")
}
fn make_feature(json: &Json) -> model::Feature {
    use rustc_serialize::json::Json::*;
    use mdo::option::{bind, ret};

    let street = mdo! {
        s =<< json.find("street");
        s =<< s.find("street_name");
        ret ret(s.clone())
    }.unwrap_or(Null);
    let house_number = mdo! {
        nb =<< json.find("house_number");
        ret ret(nb.clone())
    }.unwrap_or(Null);
    let name = mdo! {
        let house_number = &house_number;
        let street = &street;
        nb =<< house_number.as_string();
        s =<< street.as_string();
        ret ret(String(format!("{} {}", nb, s)))
    }.unwrap_or(Null);

    model::Feature {
        feature_type: "Feature".to_string(),
        properties: vec![
            model::Property {
                label: get_val(json, &["name"])
            }
        ],
        geometry: vec![
            model::Geometry {

            }
        ]
    }

    make_obj(vec![
        ("properties", make_obj(vec![
            ("label", json.find("name")
                          .map(|j| j.clone())
                          .unwrap_or(Null)),
            ("name", name),
            ("housenumber", house_number),
            ("street", street),
            ("postcode", json.find(&["street", "administrative_region", "zip_code")
                             .map(|j| j.clone())
                             .unwrap_or(Null)),
            ("city", json.find("street")
                         .and_then(|s| s.find("administrative_region"))
                         .and_then(|s| s.find("name"))
                         .map(|j| j.clone())
                         .unwrap_or(Null)),
            ("country", String("France".to_string()))
            ])),
        ("type", String("Feature".to_string())),
        ("geometry", make_obj(vec![
            ("type", String("Point".to_string())),
            ("coordinates", Array(vec![
                json.find("coord")
                    .and_then(|j| j.find("lon"))
                    .map(|j| j.clone())
                    .unwrap_or(Null),
                json.find("coord")
                    .and_then(|j| j.find("lat"))
                    .map(|j| j.clone())
                    .unwrap_or(Null)
                ]))
            ])),
        ])
}
*/


pub fn make_autocomplete(q: String, json: &Json) -> Result<model::Autocomplete, ()> {
    /*let sources: Vec<_> = json.find("hits")
        .and_then(|hs| hs.find("hits"))
        .and_then(|hs| hs.as_array())
        .map(|hs| hs.iter()
             .filter_map(|h| h.find("_source"))
             .map(|s| make_feature(s)))
        .unwrap()
        .collect();*/

    let sources = Vec::<model::Feature>::new();
    Ok(model::Autocomplete::new(q, sources))
}

pub fn autocomplete(q: String, coord: Option<model::Coord>) -> Result<model::Autocomplete, ()> {
    let raw_es = if let Some(ref coord) = coord {
        query_location(&q, coord)
    } else {
        query(&q)
    }.unwrap();
    let es = Json::from_str(str::from_utf8(raw_es.get_body()).unwrap()).unwrap();

    make_autocomplete(q, &es)
}
