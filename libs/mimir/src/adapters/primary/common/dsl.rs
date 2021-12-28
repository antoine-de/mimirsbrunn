use crate::adapters::primary::common::settings::{BuildWeight, ImportanceQueryBoosts, StringQuery};
use geojson::Geometry;
use serde_json::json;
use std::collections::BTreeMap;

use super::coord::Coord;
use super::{filters, settings};

pub fn build_query(
    q: &str,
    filters: filters::Filters,
    lang: &str,
    settings: &settings::QuerySettings,
) -> serde_json::Value {
    let type_query = build_place_type_boost(&settings.type_query.boosts);
    let string_query = build_string_query(q, lang, &settings.string_query);
    let boosts = build_boosts(q, settings, &filters);
    let mut filters_poi = build_filters(filters.shape, filters.poi_types, filters.zone_types);
    let filters = vec![build_house_number_condition(q), build_matching_condition(q)]
        .append(filters_poi.as_mut());
    json!({
        "query": {
            "bool": {
                "must": [ type_query, string_query ],
                "should": boosts,
                "filter": {
                    "bool": {
                        "must": filters
                    }
                }
            }
        }
    })
}

fn build_string_query(q: &str, lang: &str, settings: &StringQuery) -> serde_json::Value {
    json!({
        "bool": {
            "boost": settings.global,
            "should": [
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["name", format!("names.{}", lang)],
                        "boost": settings.boosts.name
                    }
                },
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["label", format!("label.{}", lang)],
                        "boost": settings.boosts.label
                    }
                },
                {
                    "multi_match": {
                        "query": q,
                        "fields": ["label.prefix", format!("label.prefix.{}", lang)],
                        "boost": settings.boosts.label_prefix
                    }
                },
                {
                    "match": {
                        "zip_codes": {
                        "query": q,
                        "boost": settings.boosts.zip_codes
                        }
                    }

                },
                {
                    "match": {
                        "house_number": {
                         "query": q,
                         "boost": settings.boosts.house_number
                        }
                    }
                }
            ]
        }
    })
}

fn build_boosts(
    _q: &str,
    settings: &settings::QuerySettings,
    filters: &filters::Filters,
) -> Vec<serde_json::Value> {
    let mut boosts: Vec<Option<serde_json::Value>> = Vec::new();
    // TODO: in production, admins are boosted by their weight only in prefix mode.
    let admin_weight_boost = Some(build_admin_weight_query(
        &settings.importance_query,
        &filters.coord,
    ));
    boosts.push(admin_weight_boost);
    let mut decay = settings.importance_query.proximity.decay.clone();
    if let Some(proximity) = &filters.proximity {
        decay.scale = proximity.scale;
        decay.offset = proximity.offset;
        decay.decay = proximity.decay;
    }
    let proximity_boost = filters
        .coord
        .clone()
        .map(|coord| build_proximity_boost(coord, &decay));
    boosts.push(proximity_boost);
    boosts.into_iter().flatten().collect()
}

fn build_filters(
    shape: Option<(Geometry, Vec<String>)>,
    poi_types: Option<Vec<String>>,
    zone_types: Option<Vec<String>>,
) -> Vec<serde_json::Value> {
    let mut filters: Vec<serde_json::Value> = Vec::new();
    if let Some(geoshape_filter) = shape.map(|(geometry, scope)| build_shape_query(geometry, scope))
    {
        filters.push(geoshape_filter);
    };
    if let Some(poi_types_filter) = poi_types.map(build_poi_types_filter) {
        filters.push(poi_types_filter);
    }
    if let Some(zone_types_filter) = zone_types.map(build_zone_types_filter) {
        filters.push(zone_types_filter);
    }
    filters
}

fn build_weight_depending_on_radius(
    importance_query_settings: &ImportanceQueryBoosts,
    coord: &Option<Coord>,
) -> BuildWeight {
    let settings_weight = importance_query_settings.clone().weights;

    // Weights for minimal radius
    let min_weights = settings_weight.clone().min_radius_prefix;

    // Weights for maximal radius
    let max_weights = settings_weight.clone().max_radius;

    // Compute a linear combination of `min_weights` and `max_weights` depending of
    // the level of zoom.
    let zoom_ratio = match coord {
        None => 1.,
        Some(_) => {
            let (min_radius, max_radius) = settings_weight.radius_range;
            let curve = importance_query_settings.clone().proximity.decay;
            let radius = (curve.offset + curve.scale).min(max_radius).max(min_radius);
            (radius.ln_1p() - min_radius.ln_1p()) / (max_radius.ln_1p() - min_radius.ln_1p())
        }
    };

    BuildWeight {
        admin: (1. - zoom_ratio) * min_weights.admin + zoom_ratio * max_weights.admin,
        factor: (1. - zoom_ratio) * min_weights.factor + zoom_ratio * max_weights.factor,
        missing: (1. - zoom_ratio) * min_weights.missing + zoom_ratio * max_weights.missing,
    }
}

fn build_house_number_condition(q: &str) -> serde_json::Value {
    if q.split_whitespace().count() > 1 {
        // Filter to handle house number.
        // We either want:
        // * to exactly match the document house_number
        // * or that the document has no house_number
        json!({
            "bool": {
                "should": [
                {
                    "bool": {
                        "must_not": {
                            "exists": {
                              "field": "house_number"
                            }
                        },
                    }
                },
                {
                    "match": {
                        "house_number": {
                            "query": q
                        }
                    }
                }
                ]
            }
        })
    } else {
        // If the query contains a single word, we don't exact any house number in the result.
        json!({
            "bool": {
                "must_not": {
                    "exists": {
                        "field": "house_number"
                    }
                },
            }
        })
    }
}

fn build_matching_condition(q: &str) -> serde_json::Value {
    // Filter to handle house number.
    // We either want:
    // * to exactly match the document house_number
    // * or that the document has no house_number
    json!({
        "match": {
            "full_label.prefix": {
                "query": q,
                "operator": "and"
            }
        }
    })
}

fn build_admin_weight_query(
    settings: &settings::ImportanceQueryBoosts,
    coord: &Option<Coord>,
) -> serde_json::Value {
    let weights = build_weight_depending_on_radius(settings, coord);
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
                    "weight": weights.admin
                }
            ]
        }
    })
}

fn build_place_type_boost(settings: &settings::Types) -> serde_json::Value {
    json!({
        "bool": {
            "should": [
                { "term": { "type": { "value": "admin", "boost": settings.admin } } },
                { "term": { "type": { "value": "addr", "boost": settings.address } } },
                { "term": { "type": { "value": "stop", "boost": settings.stop } } },
                { "term": { "type": { "value": "poi", "boost": settings.poi } } },
                { "term": { "type": { "value": "street", "boost": settings.street } } },
            ]
        }
    })
}

fn build_proximity_boost(coord: Coord, settings_decay: &settings::Decay) -> serde_json::Value {
    let settings::Decay {
        func,
        scale,
        offset,
        decay,
    } = settings_decay;

    json!({
        "function_score": {
            func.clone(): {
                "coord": {
                    "origin": {
                        "lat": coord.lat,
                        "lon": coord.lon
                    },
                    "scale": format!("{}km", scale),
                    "offset": format!("{}km", offset),
                    "decay": decay
                }
            }
        }
    })
}

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
                            "type": scope
                        }
                    },
                    "filter": {
                        "geo_shape": {
                            "approx_coord": {
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
                            "type": scope
                        }
                    }
                }
            }
            ]
        }
    })
}

// If we search for POIs and we specify poi_types, then we add a filter that should say something
// like:
// If the place is a POI, then its poi_type must be part of the given list
// So if poi_types = {A, B}, we should end up with something like
// should [
//   must {               => for pois, filter their poi types
//     type: poi
//     filter: poi_types = {A, B}
//   },
//   must_not {            => or don't filter for poi types on other places
//      type: poi
//   }
// ]
pub fn build_poi_types_filter(poi_types: Vec<String>) -> serde_json::Value {
    json!({
        "bool": {
            "should": [
            {
                "bool": {
                    "must": {
                        "term": {
                            "_source.type": "poi"
                        }
                    },
                    "filter": {
                        "terms": {
                            "_source.poi_type.id": poi_types
                        }
                    }
                }
            },
            {
                "bool": {
                    "must_not": {
                        "term": {
                            "_source.type": "poi"
                        }
                    }
                }
            }
            ]
        }
    })
}

// If we search for administrative regions and we specify zone_types, then we add a filter that should say something
// like:
// If the place is an administrative region, then its zone_type must be part of the given list
// So if zone_type = {A, B}, we should end up with something like
// should [
//   must {               => for admins, make sure they have the right zone type
//     type: admin
//     filter: zone_types
//   },
//   must_not {            => no filter on zone types for other places
//      type: admin
//   }
// ]
//
//
pub fn build_zone_types_filter(zone_types: Vec<String>) -> serde_json::Value {
    json!({
        "bool": {
            "should": [
                {
                    "bool": {
                        "must": {
                            "term": {
                                "type": "admin"
                            }
                        },
                        "filter": {
                            "terms": {
                                "zone_type":zone_types
                            }
                        }
                    }
                },
                {
                    "bool": {
                        "must_not": {
                            "term": {
                                "type": "admin"
                            }
                        }
                    }
                }
            ]
        }
    })
}

pub fn build_features_query(indices: &[String], doc_id: &str) -> serde_json::Value {
    let vec: Vec<BTreeMap<&str, &str>> = indices
        .iter()
        .map(|index| {
            vec![("_index", index.as_str()), ("_id", doc_id)]
                .into_iter()
                .collect()
        })
        .collect();
    json!({ "docs": vec })
}

// fn build_with_weight(build_weight: BuildWeight, types: &Types) -> serde_json::Value {
//     json!({
//         "function_score": {
//             "boost_mode": "replace",
//             "functions": [
//                 {
//                         "query": { "term": { "_type": "admin" } },
//                         "field_value_factor": {
//                             "field": "weight",
//                             "factor": build_weight.factor,
//                             "missing": build_weight.missing
//                     }
//                 },
//                 {
//                         "query": { "term": { "_type": "address" } },
//                         "field_value_factor": {
//                             "field": "weight",
//                             "factor": build_weight.factor,
//                             "missing": build_weight.missing
//                         }
//                 },
//                                 {
//                         "query": { "term": { "_type": "admin" } },
//                         "field_value_factor": {
//                             "field": "weight",
//                             "factor": build_weight.factor,
//                             "missing": build_weight.missing
//                         }
//                 },
//                                 {
//                         "query": { "term": { "_type": "poi" } },
//                         "field_value_factor": {
//                             "field": "weight",
//                             "factor": build_weight.factor,
//                             "missing": build_weight.missing
//                     }
//                 },
//                 {
//                         "query": { "term": { "_type": "street" } },
//                         "field_value_factor": {
//                             "field": "weight",
//                             "factor": build_weight.factor,
//                             "missing": build_weight.missing
//                     }
//                 }
//             ]
//         }
//     })
// }

//
// fn build_coverage_condition() -> serde_json::Value {
//     // filter to handle PT coverages
//     // we either want:
//     // * to get objects with no coverage at all (non-PT objects)
//     // * or the objects with coverage matching the ones we're allowed to get
//     json!({
//             "bool": {
//                 "should": [
//                 {
//                     "bool": {
//                         "must_not": {
//                             "exists": {
//                               "field": "coverages"
//                             }
//                         },
//                     }
//                 },
//                 {
//                     "term": {
//                         "coverages": []
//                     }
//                 }
//             ]
//         }
//     })
// }

// fn build_search_as_you_type_query(q: &str, settings: &settings::StringQuery) -> serde_json::Value {
//     json!({
//         "bool": {
//             "boost": settings.global,
//             "should": [
//                 {
//                     "multi_match": {
//                         "query": q,
//                         "type": "bool_prefix", // match_phrase_prefix query match terms order
//                         "fields": [
//                             "label", "label._2gram", "label._3gram", "name"
//                         ]
//                     }
//                 }
//             ]
//         }
//     })
// }
