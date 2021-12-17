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
        zone_types,
        poi_types,
        timeout: _,
    } = filters;

    let string_query = build_string_query(q, &settings.string_query);
    let boosts = build_boosts(q, settings, coord);
    let filters = build_filters(shape, poi_types, zone_types);
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

fn build_filters(
    shape: Option<(Geometry, Vec<String>)>,
    poi_types: Option<Vec<String>>,
    zone_types: Option<Vec<String>>,
) -> Vec<serde_json::Value> {
    let mut filters: Vec<Option<serde_json::Value>> = Vec::new();
    let geoshape_filter = shape.map(|(geometry, scope)| build_shape_query(geometry, scope));
    filters.push(geoshape_filter);
    let poi_types_filter = poi_types.map(build_poi_types_filter);
    filters.push(poi_types_filter);
    let zone_types_filter = zone_types.map(build_zone_types_filter);
    filters.push(zone_types_filter);
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
    let place_type_boost = Some(build_place_type_boost(&settings.type_query.boosts));
    boosts.push(place_type_boost);
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
                            "_source.type": scope
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
                            "_source.type": scope
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
