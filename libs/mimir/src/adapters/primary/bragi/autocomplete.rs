use super::settings::{ImportanceQueryBoosts, QuerySettings, StringQuery};
use serde_json::json;

#[derive(Debug)]
pub struct Coord {
    pub lat: f32,
    pub lon: f32,
}

impl Coord {
    pub fn new(lat: f32, lon: f32) -> Self {
        Coord { lat, lon }
    }
}

/* How to restrict the range of the query... */
#[derive(Debug, Default)]
pub struct Filters {
    pub coord: Option<Coord>,
    pub shape: Option<(String, Vec<String>)>,
    pub datasets: Option<Vec<String>>,
    pub zone_types: Option<Vec<String>>,
    pub poi_types: Option<Vec<String>>,
}

pub fn build_query(
    q: &str,
    _filters: Filters,
    _langs: &[&str],
    settings: &QuerySettings,
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

fn build_string_query(q: &str, settings: &StringQuery) -> serde_json::Value {
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

fn build_importance_query(_q: &str, settings: &QuerySettings) -> serde_json::Value {
    // TODO: in production, admins are boosted by their weight only in prefix mode.
    json!([build_admin_weight_query(&settings.importance_query)])
}

fn build_admin_weight_query(settings: &ImportanceQueryBoosts) -> serde_json::Value {
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
