use super::{filters, settings};
use common::document::ContainerDocument;
use serde_json::json;

pub fn build_query(
    q: &str,
    _filters: filters::Filters,
    _langs: &[&str],
    settings: &settings::QuerySettings,
) -> serde_json::Value {
    json!({
        "query": {
            "bool": {
                "must": [ build_string_query(q, &settings.string_query) ],
                "should": build_importance_query(q, settings)
            },
        }
    })
}

fn build_string_query(q: &str, settings: &settings::StringQuery) -> serde_json::Value {
    json!({
        "bool": {
            "boost": settings.global,
            "should": [
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["name"],
                        "boost": settings.boosts.name
                    }
                },
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["label"],
                        "boost": settings.boosts.label
                    }
                },
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["label.prefix"],
                        "boost": settings.boosts.label_prefix
                    }
                }
            ]
        }
    })
}

fn build_importance_query(_q: &str, settings: &settings::QuerySettings) -> serde_json::Value {
    json!([
        build_with_weight(
            &settings.importance_query.weights.max_radius, // TODO: take distance into account
            &settings.importance_query.weights.types // TODO: in production, admins are boosted by their weight only in prefix mode.
        ),
        build_admin_weight_query(&settings.importance_query)
    ])
}

fn build_with_weight(
    build_weight: &settings::BuildWeight,
    types: &settings::Types,
) -> serde_json::Value {
    let weighted = |doc_type, weight| {
        json!({
            "filter": { "term": { "type": doc_type } },
            "weight": weight,
            "field_value_factor": {
                "field": "weight",
                "factor": build_weight.factor,
                "missing": build_weight.missing
            }
        })
    };

    // let weighted = |doc_type, weight| {
    //     FilteredFunction::build_filtered_function(
    //         Query::build_term("_type", doc_type).build(),
    //         Function::build_field_value_factor("weight")
    //             .with_factor(build_weight.factor)
    //             .with_missing(build_weight.missing)
    //             .build(),
    //         Function::build_weight(weight),
    //     )
    // };

    // Query::build_function_score()
    //     .with_functions(vec![
    //         weighted(Stop::doc_type(), types.stop),
    //         weighted(Addr::doc_type(), types.address),
    //         weighted(Admin::doc_type(), types.admin),
    //         weighted(Poi::doc_type(), types.poi),
    //         weighted(Street::doc_type(), types.street),
    //     ])
    //     .with_boost_mode(BoostMode::Replace)
    //     .build()
    json!({
        "function_score": {
            "boost_mode": "replace",
            "functions": [
                weighted(places::admin::Admin::static_doc_type(), types.admin),
                weighted(places::addr::Addr::static_doc_type(), types.address)
            ]
        }
    })
}

fn build_admin_weight_query(settings: &settings::ImportanceQueryBoosts) -> serde_json::Value {
    json!({
        "function_score": {
            "query": { "term": { "type": "admin" } },
            "boost_mode": "replace",
            "functions": [
                {
                    "field_value_factor": {
                        "field": "weight",
                        "factor": 1e6,
                        "modifier": "log1p",
                        "missing": 0
                    }
                },
                {
                    // TODO: in production, this weight depends of the focus radius
                    "weight": settings.weights.max_radius.admin
                }
            ]
        }
    })
}

// fn build_coverage_condition(datasets: &[&str]) -> serde_json::Value {
//     json!({
//         "bool": {
//             "should": [
//                 { "bool": { "must_not": { "exists": { "field": "coverages" } } } },
//                 { "terms": { "coverages": datasets } }
//             ]
//         }
//     })
// }
//
// #[derive(Debug)]
// enum DecayFn {
//     Gauss,
//     Exp,
//     Linear,
// }
//
// impl std::fmt::Display for DecayFn {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "{:?}", self)
//     }
// }
//
// #[derive(Debug)]
// struct DecayFnParam {
//     pub func: DecayFn,
//     pub scale: f32,
//     pub offset: f32,
//     pub decay: f32,
// }

// // The decay function takes 4 parameters:
// // - origin: Here the origin is a geo-point, so we can express it as an object:
// //    { "location": { "lat": ..., "lon": ... } }
// // - scale
// // - offset
// // - decay
// // FIXME Probably use something like Into<Coord>
// fn build_proximity_with_boost(coord: Coord, decay_fn_param: DecayFnParam) -> String {
//     format!(
//         r#"{{
//             "function_score": {{
//                 "{func}": {{
//                     "coord": {{
//                         "origin": {{
//                             "location": {{
//                                 "lat": {lat},
//                                 "lon": {lon}
//                             }}
//                         }},
//                         "scale": "{scale}km",
//                         "offset": "{offset}km",
//                         "decay": {decay}
//                     }}
//                 }}
//             }}
//         }}"#,
//         lat = coord.lat,
//         lon = coord.lon,
//         func = decay_fn_param.func,
//         scale = decay_fn_param.scale,
//         offset = decay_fn_param.offset,
//         decay = decay_fn_param.decay
//     )
// }

pub fn build_reverse_query(distance: &str, lat: f64, lon: f64) -> serde_json::Value {
    json!({
    "query": {
        "bool": {
            "must": {
                "match_all": {}
            },
            "filter": {
                "geo_distance": {
                    "distance": distance,
                    "coord": {
                        "lat": lat,
                        "lon": lon
                    }
                }
            }
        }
    },
    "sort": [{
         "_geo_distance": {
             "coord": {
                 "lat": lat,
                 "lon": lon
             },
             "order": "asc",
             "unit": "m",
             "mode": "min",
             "distance_type": "arc",
             "ignore_unmapped": true
         }
     }]
    })
}
