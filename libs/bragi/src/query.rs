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
use rs_es;
use rs_es::error::EsError;
use rs_es::query::Query;
use rs_es::operations::search::SearchResult;
use rs_es::units as rs_u;
use mimir;
use serde_json;
use serde;
use mimir::objects::{Addr, Admin, MimirObject, Poi, Stop, Street};

/// takes a ES json blob and build a Place from it
/// it uses the _type field of ES to know which type of the Place enum to fill
pub fn make_place(doc_type: String, value: Option<Box<serde_json::Value>>) -> Option<mimir::Place> {
    value.and_then(|v| {
        fn convert<T>(v: serde_json::Value, f: fn(T) -> mimir::Place) -> Option<mimir::Place>
        where
            for<'de> T: serde::Deserialize<'de>,
        {
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

#[derive(Debug, Eq, PartialEq)]
enum MatchType {
    Prefix,
    Fuzzy,
}

/// Create a `rs_es::Query` that boosts results according to the
/// distance to `coord`.
fn build_proximity_with_boost(coord: &model::Coord, boost: f64) -> Query {
    Query::build_function_score()
        .with_boost(boost)
        .with_function(
            rs_es::query::functions::Function::build_decay(
                "coord",
                rs_u::Location::LatLon(coord.lat, coord.lon),
                rs_u::Distance::new(50f64, rs_u::DistanceUnit::Kilometer),
            ).build_gauss(),
        )
        .build()
}

// filter to handle PT coverages
// we either want:
// * to get objects with no coverage at all (non-PT objects)
// * or the objects with coverage matching the ones we're allowed to get
fn build_coverage_condition(pt_datasets: &[&str]) -> Query {
    Query::build_bool()
        .with_should(vec![
            Query::build_bool()
                .with_must_not(Query::build_exists("coverages").build())
                .build(),
            Query::build_terms("coverages")
                .with_values(pt_datasets)
                .build(),
        ])
        .build()
}

fn build_query(
    q: &str,
    match_type: MatchType,
    coord: &Option<model::Coord>,
    shape: Option<Vec<rs_es::units::Location>>,
    pt_datasets: &[&str],
    all_data: bool,
) -> Query {
    use rs_es::query::functions::Function;

    // Priorization by type
    fn match_type_with_boost<T: MimirObject>(boost: f64) -> Query {
        Query::build_term("_type", T::doc_type())
            .with_boost(boost)
            .build()
    }
    let type_query = Query::build_bool()
        .with_should(vec![
            match_type_with_boost::<Addr>(12.),
            match_type_with_boost::<Admin>(11.),
            match_type_with_boost::<Stop>(10.),
            match_type_with_boost::<Poi>(2.),
            match_type_with_boost::<Street>(1.),
        ])
        .with_boost(20.)
        .build();

    // Priorization by query string
    let mut string_should = vec![
        Query::build_match("label", q).with_boost(1.).build(),
        Query::build_match("label.prefix", q).with_boost(1.).build(),
        Query::build_match("zip_codes", q).with_boost(1.).build(),
    ];
    if let MatchType::Fuzzy = match_type {
        string_should.push(Query::build_match("label.ngram", q).with_boost(1.).build());
    }
    let string_query = Query::build_bool()
        .with_should(string_should)
        .with_boost(1.)
        .build();

    // Priorization by importance
    let importance_query = match coord {
        &Some(ref c) => build_proximity_with_boost(c, 100.),
        &None => Query::build_function_score()
            .with_function(Function::build_field_value_factor("weight").build())
            .with_boost(30.)
            .build(),
    };

    // filter to handle house number
    // we either want:
    // * to exactly match the document house_number
    // * or that the document has no house_number
    let house_number_condition = Query::build_bool()
        .with_should(vec![
            Query::build_bool()
                .with_must_not(Query::build_exists("house_number").build())
                .build(),
            Query::build_match("house_number", q.to_string()).build(),
        ])
        .build();

    use rs_es::query::MinimumShouldMatch;
    use rs_es::query::CombinationMinimumShouldMatch;

    let matching_condition = match match_type {
        // When the match type is Prefix, we want to use every possible information even though
        // these are not present in label, for instance, the zip_code.
        // The field full_label contains all of them and will do the trick.
        MatchType::Prefix => Query::build_match("full_label.prefix".to_string(), q.to_string())
            .with_operator("and")
            .build(),
        // for fuzzy search we lower our expectation & we accept a certain percentage of token match
        // on full_label.ngram
        // The values defined here are empirical,
        // it's supposed to be able to manage cases BOTH missspelt one-word
        // www.elastic.co/guide/en/elasticsearch/guide/current/match-multi-word.html#match-precision
        // requests AND very long requests.
        // Missspelt one-word request:
        //     Vaureaaal (instead of Vaureal)
        // Very long requests:
        //     Caisse Primaire d'Assurance Maladie de Haute Garonne, 33 Rue du Lot, 31100 Toulouse
        MatchType::Fuzzy => Query::build_match("full_label.ngram".to_string(), q.to_string())
            .with_minimum_should_match(MinimumShouldMatch::from(vec![
                CombinationMinimumShouldMatch::new(1i64, 75f64),
                CombinationMinimumShouldMatch::new(6i64, 60f64),
                CombinationMinimumShouldMatch::new(9i64, 40f64),
            ]))
            .build(),
    };

    let mut filters = vec![house_number_condition, matching_condition];

    // if searching through all data, no coverage filter
    if !all_data {
        filters.push(build_coverage_condition(pt_datasets));
    }

    if let Some(s) = shape {
        filters.push(Query::build_geo_polygon("coord", s).build());
    }

    Query::build_bool()
        .with_must(vec![type_query, string_query, importance_query])
        .with_filter(Query::build_bool().with_must(filters).build())
        .build()
}

fn is_existing_index(client: &mut rs_es::Client, index: &str) -> Result<bool, EsError> {
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

fn get_indexes_by_type(a_type: &str) -> String {
    let doc_type = match a_type {
        "public_transport:stop_area" => "stop",
        "city" => "admin",
        "house" => "addr",
        _ => a_type,
    };

    format!("munin_{}", doc_type)
}

fn make_indexes_impl<F: FnMut(&str) -> Result<bool, EsError>>(
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

fn make_indexes(
    all_data: bool,
    pt_datasets: &[&str],
    types: &[&str],
    client: &mut rs_es::Client,
) -> Result<Vec<String>, EsError> {
    make_indexes_impl(all_data, pt_datasets, types, |index| {
        is_existing_index(client, index)
    })
}

fn collect(result: SearchResult<serde_json::Value>) -> Result<Vec<mimir::Place>, EsError> {
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

fn query(
    q: &str,
    pt_datasets: &[&str],
    all_data: bool,
    cnx: &str,
    match_type: MatchType,
    offset: u64,
    limit: u64,
    coord: &Option<model::Coord>,
    shape: Option<Vec<rs_es::units::Location>>,
    types: &[&str],
) -> Result<Vec<mimir::Place>, EsError> {
    let query = build_query(q, match_type, coord, shape, pt_datasets, all_data);

    let mut client = rs_es::Client::new(cnx).unwrap();

    let indexes = try!(make_indexes(all_data, &pt_datasets, types, &mut client));

    debug!("ES indexes: {:?}", indexes);

    if indexes.is_empty() {
        // if there is no indexes, rs_es search with index "_all"
        // but we want to return empty response in this case.
        return Ok(vec![]);
    }

    let result: SearchResult<serde_json::Value> = try!(
        client
            .search_query()
            .with_indexes(&indexes
                .iter()
                .map(|index| index.as_str())
                .collect::<Vec<&str>>())
            .with_query(&query)
            .with_from(offset)
            .with_size(limit)
            .send()
    );

    collect(result)
}

pub fn features(
    pt_datasets: &[&str],
    all_data: bool,
    cnx: &str,
    id: &str,
) -> Result<Vec<mimir::Place>, EsError> {
    let val = rs_es::units::JsonVal::String(id.into());
    let mut filters = vec![Query::build_ids(vec![val]).build()];

    // if searching through all data, no coverage filter
    if !all_data {
        filters.push(build_coverage_condition(pt_datasets));
    }
    let filter = Query::build_bool().with_must(filters).build();
    let query = Query::build_bool().with_filter(filter).build();

    let mut client = rs_es::Client::new(cnx).unwrap();

    let indexes = try!(make_indexes(all_data, &pt_datasets, &[], &mut client));

    debug!("ES indexes: {:?}", indexes);

    if indexes.is_empty() {
        // if there is no indexes, rs_es search with index "_all"
        // but we want to return an error in this case.
        return Err(EsError::EsError("Unable to find object".to_string()));
    }

    let result: SearchResult<serde_json::Value> = try!(
        client
            .search_query()
            .with_indexes(&indexes
                .iter()
                .map(|index| index.as_str())
                .collect::<Vec<&str>>())
            .with_query(&query)
            .send()
    );

    if result.hits.total == 0 {
        Err(EsError::EsError("Unable to find object".to_string()))
    } else {
        collect(result)
    }
}

pub fn autocomplete(
    q: &str,
    pt_datasets: &[&str],
    all_data: bool,
    offset: u64,
    limit: u64,
    coord: Option<model::Coord>,
    cnx: &str,
    shape: Option<Vec<(f64, f64)>>,
    types: &[&str],
) -> Result<Vec<mimir::Place>, EsError> {
    fn make_shape(shape: &Option<Vec<(f64, f64)>>) -> Option<Vec<rs_es::units::Location>> {
        shape
            .as_ref()
            .map(|v| v.iter().map(|&l| l.into()).collect())
    }

    // First we try a pretty exact match on the prefix.
    // If there are no results then we do a new fuzzy search (matching ngrams)
    let results = try!(query(
        &q,
        &pt_datasets,
        all_data,
        cnx,
        MatchType::Prefix,
        offset,
        limit,
        &coord,
        make_shape(&shape),
        &types,
    ));
    if results.is_empty() {
        query(
            &q,
            &pt_datasets,
            all_data,
            cnx,
            MatchType::Fuzzy,
            offset,
            limit,
            &coord,
            make_shape(&shape),
            &types,
        )
    } else {
        Ok(results)
    }
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
