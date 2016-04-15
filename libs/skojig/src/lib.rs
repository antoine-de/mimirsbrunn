// Copyright © 2014, Canal TP and/or its affiliates. All rights reserved.
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

extern crate csv;
extern crate rustc_serialize;
extern crate curl;
extern crate docopt;
extern crate iron;
extern crate urlencoded;

use iron::headers::ContentType;
use iron::prelude::*;
use iron::status;
use urlencoded::UrlEncodedQuery;
use rustc_serialize::json::Json;
use rustc_serialize::Encodable;

#[macro_use] extern crate mdo;

#[derive(RustcDecodable, RustcEncodable)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

pub type CurlResult = Result<curl::http::Response, curl::ErrCode>;

fn query(q: &str) -> Result<curl::http::Response, curl::ErrCode> {
    use rustc_serialize::json::Json::String;
    let query = format!(include_str!("../../..//json/query_exact.json"), query=String(q.to_string()));
    let resp = try!(curl::http::handle()
                    .post("http://localhost:9200/munin/_search?pretty", &query)
                    .exec());
    let body = Json::from_str(std::str::from_utf8(resp.get_body()).unwrap()).unwrap();
    if body["hits"]["total"].as_u64().unwrap() > 0 { return Ok(resp); }
    let query = format!(include_str!("../../../json/query.json"), query=String(q.to_string()));
    let resp = curl::http::handle()
        .post("http://localhost:9200/munin/_search?pretty", &query)
        .exec();
    resp
}

fn query_location(q: &str, coord: &Coord) -> Result<curl::http::Response, curl::ErrCode> {
    use rustc_serialize::json::Json::String;
    let query = format!(include_str!("../../../json/query_exact_location.json"),
                        query=String(q.to_string()),
                        lon=coord.lon,
                        lat=coord.lat);
    let resp = try!(curl::http::handle()
                    .post("http://localhost:9200/munin/_search?pretty", &query)
                    .exec());
    let body = Json::from_str(std::str::from_utf8(resp.get_body()).unwrap()).unwrap();
    if body["hits"]["total"].as_u64().unwrap() > 0 { return Ok(resp); }
    let query = format!(include_str!("../../../json/query_location.json"),
                        query=String(q.to_string()),
                        lon=coord.lon,
                        lat=coord.lat);
    let resp = curl::http::handle()
        .post("http://localhost:9200/munin/_search?pretty", &query)
        .exec();
    resp
}

fn make_obj(v: Vec<(&'static str, Json)>) -> Json {
    use rustc_serialize::json::Json::Object;
    Object(v.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}


fn make_feature(json: &Json) -> Json {
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
    make_obj(vec![
        ("properties", make_obj(vec![
            ("label", json.find("name")
                          .map(|j| j.clone())
                          .unwrap_or(Null)),
            ("name", name),
            ("housenumber", house_number),
            ("street", street),
            ("postcode", json.find("street")
                             .and_then(|s| s.find("administrative_region"))
                             .and_then(|s| s.find("zip_code"))
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

fn handle_query(req: &mut Request) -> IronResult<Response> {
    use rustc_serialize::json::Json::*;
    use mdo::option::{bind, ret};

    let map = req.get_ref::<UrlEncodedQuery>().unwrap();
    let q = map.get("q").and_then(|v| v.get(0)).unwrap();
    let coord = mdo! {
        lons =<< map.get("lon");
        lon =<< lons.get(0);
        lon =<< std::str::FromStr::from_str(lon).ok();
        lats =<< map.get("lat");
        lat =<< lats.get(0);
        lat =<< std::str::FromStr::from_str(lat).ok();
        ret ret(Coord { lon: lon, lat: lat })
    };
    let es = if let Some(ref coord) = coord {
        query_location(q, coord)
    } else {
        query(q)
    }.unwrap();
    let es = Json::from_str(std::str::from_utf8(es.get_body()).unwrap()).unwrap();
    let sources: Vec<_> = es.find("hits")
        .and_then(|hs| hs.find("hits"))
        .and_then(|hs| hs.as_array())
        .map(|hs| hs.iter()
             .filter_map(|h| h.find("_source"))
             .map(|s| make_feature(s)))
        .unwrap()
        .collect();
    let json = make_obj(vec![
        ("type", String("FeatureCollection".to_string())),
        ("version", String("0.1.0".to_string())),
        ("query", String(q.clone())),
        ("features", Array(sources))
    ]);

    let mut resp = Response::with((status::Ok, format!("{}", json.pretty())));
    resp.headers.set(ContentType("application/json".parse().unwrap()));
    Ok(resp)
}

pub fn runserver() {
    Iron::new(handle_query).http("0.0.0.0:3000").unwrap();
}
