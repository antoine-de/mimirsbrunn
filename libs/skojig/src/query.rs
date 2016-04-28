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

pub type CurlResult = Result<curl::http::Response, curl::ErrCode>;

fn query(q: &String) -> Result<curl::http::Response, curl::ErrCode> {
    use rustc_serialize::json::Json::String;
    let query = format!(include_str!("../../../json/query_exact.json"), query=String(q.to_string()));
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
