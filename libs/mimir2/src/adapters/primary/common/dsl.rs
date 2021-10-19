use geojson::Geometry;
use serde_json::json;

use super::{filters, settings};

pub fn build_query(
    q: &str,
    filters: filters::Filters,
    _langs: &[&str],
    settings: &settings::QuerySettings,
) -> serde_json::Value {
    let string_query = build_string_query(q, &settings.string_query);
    let importance_query = build_importance_query(q, settings);

    let filters::Filters {
        coord: _,
        shape,
        datasets: _,
        zone_types: _,
        poi_types: _,
    } = filters;

    let geoshape_filter = shape.map(|(geometry, scope)| build_shape_query(geometry, scope));
    let filters: Vec<_> = vec![geoshape_filter].into_iter().flatten().collect();

    if filters.is_empty() {
        json!({
            "query": {
                "bool": {
                    "must": [ string_query ],
                    "should": importance_query,
                }
            }
        })
    } else {
        json!({
            "query": {
                "bool": {
                    "must": [ string_query ],
                    "should": importance_query,
                    "filter": {
                        "bool": {
                            "must": filters
                        }
                    }
                }
            }
        })
    }
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
    // TODO: in production, admins are boosted by their weight only in prefix mode.
    json!([build_admin_weight_query(&settings.importance_query)])
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

// If there is a shape, all the places listed in shape_scope are restricted to the shape.
// and the places that are not listed are not restricted.
// So if shape_scope = {A, B}, we should end up with something like
// should [
//   must {               => filwer_w_shape
//     term: _type in {A, B}
//     filter: geoshape
//   },
//   must_not {            => filter_wo_shape
//      term: _type in {A, B}
//   }
// ]
//
pub fn build_shape_query(shape: Geometry, scope: Vec<String>) -> serde_json::Value {
    json!({
        "bool": {
            "should": [
            {
                "bool": {
                    "must": {
                        "terms": {
                            "_type": scope
                        }
                    },
                    "filter": {
                        "geo_shape": {
                            "location": {
                                "shape": shape,
                                "relation": "intersects"
                            }
                        }
                    }
                }
            },
            {
                "bool": {
                    "must_not": {
                        "terms": {
                            "_type": scope
                        }
                    }
                }
            }
            ]
        }
    })
}
