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
use super::model;
use regex;
use rs_es;
use rs_es::query::Query as rs_q;
use rs_es::operations::search::SearchResult;
use rs_es::units as rs_u;
use mimir;
use serde_json;
use serde;

fn build_rs_client(cnx: &String) -> rs_es::Client {
    let re = regex::Regex::new(r"(?:https?://)?(?P<host>.+?):(?P<port>\d+)").unwrap();
    let cap = re.captures(&cnx).unwrap();
    let host = cap.name("host").unwrap();
    let port = cap.name("port").unwrap().parse::<u32>().unwrap();

    rs_es::Client::new(&host, port)
}

/// takes a ES json blob and build a Place from it
/// it uses the _type field of ES to know which type of the Place enum to fill
pub fn make_place(doc_type: String, value: Option<Box<serde_json::Value>>) -> Option<mimir::Place> {
    value.and_then(|v| {
        fn convert<T: serde::Deserialize>(v: serde_json::Value,
                                          f: fn(T) -> mimir::Place)
                                          -> Option<mimir::Place> {
            serde_json::from_value::<T>(v)
                .map_err(|err| warn!("Impossible to load ES result: {}", err))
                .ok()
                .map(f)
        }
        match doc_type.as_ref() {
            "addr" => convert(*v, mimir::Place::Addr),
            "street" => convert(*v, mimir::Place::Street),
            "admin" => convert(*v, mimir::Place::Admin),
            "poi" => convert(*v, mimir::Place::Poi),
            "stop" => convert(*v, mimir::Place::Stop),
            _ => {
                warn!("unknown ES return value, _type field = {}", doc_type);
                None
            }
        }
    })
}

fn build_query(q: &str,
               match_type: &str,
               coord: &Option<model::Coord>,
               shape: Option<Vec<rs_es::units::Location>>)
               -> rs_es::query::Query {
    use rs_es::query::functions::Function;
    let boost_addr = rs_q::build_term("_type", "addr").with_boost(1000).build();
    let boost_match_query = rs_q::build_multi_match(vec![match_type.to_string(),
                                                         "zip_codes.prefix".to_string()],
                                                    q.to_string())
        .with_boost(100)
        .build();

    let mut should_query = vec![boost_addr, boost_match_query];
    if let &Some(ref c) = coord {
        // if we have coordinate, we boost we result near this coordinate
        let boost_on_proximity =
            rs_q::build_function_score()
                .with_boost_mode(rs_es::query::compound::BoostMode::Multiply)
                .with_boost(500)
                .with_function(Function::build_decay("coord",
                                           rs_u::Location::LatLon(c.lat, c.lon),
                                           rs_u::Distance::new(50f64,
                                                               rs_u::DistanceUnit::Kilometer))
                                   .build_exp())
                .build();
        should_query.push(boost_on_proximity);
    } else {
        // if we don't have coords, we take the field `weight` into account
        let boost_on_weight = rs_q::build_function_score()
            .with_boost_mode(rs_es::query::compound::BoostMode::Multiply)
            .with_boost(300)
            .with_query(rs_q::build_match_all().build())
            .with_function(Function::build_field_value_factor("weight")
                .with_factor(1)
                .with_modifier(rs_es::query::functions::Modifier::Log1p)
                .build())
            .build();
        should_query.push(boost_on_weight);
    }

    let sub_query = rs_q::build_bool()
        .with_should(should_query)
        .build();

    let mut must = vec![rs_q::build_multi_match(vec![match_type.to_string(),
        "zip_codes.prefix".to_string()], q.to_string())
             .with_minimum_should_match(rs_es::query::MinimumShouldMatch::from(90f64)).build()];

    if let Some(s) = shape {
        must.push(rs_q::build_geo_polygon("coord", s).build());
    }

    let filter = rs_q::build_bool()
        .with_should(vec![rs_q::build_bool()
                              .with_must_not(rs_q::build_exists("house_number").build())
                              .build(),
                          rs_q::build_multi_match(vec!["house_number".to_string(),
                                                       "zip_codes".to_string()],
                                                  q.to_string())
                              .build()])
        .with_must(must)
        .build();

    rs_q::build_bool()
        .with_must(vec![sub_query])
        .with_filter(filter)
        .build()
}

fn query(q: &str,
         cnx: &str,
         match_type: &str,
         offset: u64,
         limit: u64,
         coord: &Option<model::Coord>,
         shape: Option<Vec<rs_es::units::Location>>)
         -> Result<Vec<mimir::Place>, rs_es::error::EsError> {
    let query = build_query(q, match_type, coord, shape);

    let mut client = build_rs_client(&cnx.to_string());

    let result: SearchResult<serde_json::Value> = try!(client.search_query()
        .with_indexes(&["munin"])
        .with_query(&query)
        .with_from(offset)
        .with_size(limit)
        .send());

    debug!("{} documents found", result.hits.total);

    // for the moment rs-es does not handle enum Document,
    // so we need to convert the ES glob to a Place
    Ok(result.hits
        .hits
        .into_iter()
        .filter_map(|hit| make_place(hit.doc_type, hit.source))
        .collect())
}


fn query_prefix(q: &str,
                cnx: &str,
                offset: u64,
                limit: u64,
                coord: &Option<model::Coord>,
                shape: Option<Vec<rs_es::units::Location>>)
                -> Result<Vec<mimir::Place>, rs_es::error::EsError> {
    query(&q, cnx, "label.prefix", offset, limit, coord, shape)
}

fn query_ngram(q: &str,
               cnx: &str,
               offset: u64,
               limit: u64,
               coord: &Option<model::Coord>,
               shape: Option<Vec<rs_es::units::Location>>)
               -> Result<Vec<mimir::Place>, rs_es::error::EsError> {
    query(&q, cnx, "label.ngram", offset, limit, coord, shape)
}

pub fn autocomplete(q: &str,
                    offset: u64,
                    limit: u64,
                    coord: Option<model::Coord>,
                    cnx: &str,
                    shape: Option<Vec<(f64, f64)>>)
                    -> Result<Vec<mimir::Place>, rs_es::error::EsError> {
    // First search with match = "name.prefix".
    // If no result then another search with match = "name.ngram"
    fn make_shape(shape: &Option<Vec<(f64, f64)>>) -> Option<Vec<rs_es::units::Location>> {
        shape.as_ref().map(|v| v.iter().map(|&l| l.into()).collect())
    }

    let results = try!(query_prefix(&q, cnx, offset, limit, &coord, make_shape(&shape)));
    if results.is_empty() {
        query_ngram(&q, cnx, offset, limit, &coord, make_shape(&shape))
    } else {
        Ok(results)
    }
}
