use geojson::Geometry;
use serde_json::json;

use super::coord::Coord;
use super::{filters, settings};

pub fn build_query(
    q: &str,
    filters: filters::Filters,
    _langs: &[&str],
    settings: &settings::QuerySettings,
) -> serde_json::Value {
    let filters::Filters {
        coord,
        shape,
        limit: _,
        datasets: _,
        zone_types: _,
        poi_types: _,
    } = filters;

    let string_query = build_string_query(q, &settings.string_query);
    let boosts = build_boosts(q, settings, coord);
    let filters = build_filters(shape);
    if filters.is_empty() {
        json!({
            "query": {
                "bool": {
                    "must": [ string_query ],
                    "should": boosts
                }
            }
        })
    } else {
        json!({
            "query": {
                "bool": {
                    "must": [ string_query ],
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
}

fn build_string_query(q: &str, settings: &settings::StringQuery) -> serde_json::Value {
    json!({
        "bool": {
            "boost": settings.global,
            "should": [
                {
                    "multi_match": {
                        "query": q,
                        "type": "bool_prefix",
                        "fields": [
                            "label", "label._2gram", "label._3gram", "name"
                        ]
                    }
                }
            ]
        }
    })
}

fn build_filters(shape: Option<(Geometry, Vec<String>)>) -> Vec<serde_json::Value> {
    let mut filters: Vec<Option<serde_json::Value>> = Vec::new();
    let geoshape_filter = shape.map(|(geometry, scope)| build_shape_query(geometry, scope));
    filters.push(geoshape_filter);
    filters.into_iter().flatten().collect()
}

fn build_boosts(
    _q: &str,
    settings: &settings::QuerySettings,
    coord: Option<Coord>,
) -> Vec<serde_json::Value> {
    let mut boosts: Vec<Option<serde_json::Value>> = Vec::new();
    // TODO: in production, admins are boosted by their weight only in prefix mode.
    let admin_weight_boost = Some(build_admin_weight_query(&settings.importance_query));
    boosts.push(admin_weight_boost);
    let proximity_boost =
        coord.map(|coord| build_proximity_boost(coord, &settings.importance_query.proximity.decay));
    boosts.push(proximity_boost);
    boosts.into_iter().flatten().collect()
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

fn build_proximity_boost(coord: Coord, decay: &settings::Decay) -> serde_json::Value {
    let settings::Decay {
        func,
        scale,
        offset,
        decay,
    } = decay;

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