use super::settings::QuerySettings;

// fn main() {
//     let settings = include_str!("../config/settings.toml");
//     let settings = match QuerySettings::new(settings) {
//         Ok(settings) => settings,
//         Err(err) => {
//             println!("err: {}", err.to_string());
//             std::process::exit(1);
//         }
//     };
//
//     let filters = Filters {
//         coord: None,
//         shape: None,
//         datasets: None,
//         zone_types: None,
//         poi_types: None,
//     };
//
//     let query = build_query("jeanne d'arc", filters, &["fr"], &settings);
//
//     println!("{}", query);
// }
//
/* How to restrict the range of the query... */
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
) -> String {
    let query = build_query_multi_match(q, &settings);
    format!(
        r#"{{
            "query": {query}
        }}"#,
        query = query
    )
}

fn build_query_multi_match(q: &str, settings: &QuerySettings) -> String {
    format!(
        r#"{{
            "multi_match": {{
                "query": "{query}",
                "type": "bool_prefix",
                "fields": [
                  "label",
                  "label._2gram",
                  "label._3gram"
                ],
                "fuzziness": "auto",
                "boost": {boost}
            }}
        }}"#,
        query = q,
        boost = settings.string_query.boosts.label
    )
}

fn build_coverage_condition(datasets: &[&str]) -> String {
    let coverages = datasets
        .iter()
        .map(|&s| format!(r#""{}""#, s))
        .collect::<Vec<String>>();
    //.join(", ")
    format!(
        r#"{{
            "bool": {{
                "should": [
                    "bool": {{
                        "must_not": {{
                            "exists": {{
                                "field": "coverages"
                            }}
                        }}
                    }},
                    "terms": {{
                        "coverages": [{coverages}]
                    }}
                ]
            }}
        }}"#,
        coverages = coverages.join(", ")
    )
}

#[derive(Debug)]
enum DecayFn {
    Gauss,
    Exp,
    Linear,
}

impl std::fmt::Display for DecayFn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct DecayFnParam {
    pub func: DecayFn,
    pub scale: f32,
    pub offset: f32,
    pub decay: f32,
}

pub struct Coord {
    pub lat: f32,
    pub lon: f32,
}

impl Coord {
    pub fn new(lat: f32, lon: f32) -> Self {
        Coord { lat, lon }
    }
}

// The decay function takes 4 parameters:
// - origin: Here the origin is a geo-point, so we can express it as an object:
//    { "location": { "lat": ..., "lon": ... } }
// - scale
// - offset
// - decay
// FIXME Probably use something like Into<Coord>
fn build_proximity_with_boost(coord: Coord, decay_fn_param: DecayFnParam) -> String {
    format!(
        r#"{{
            "function_score": {{
                "{func}": {{
                    "coord": {{
                        "origin": {{
                            "location": {{
                                "lat": {lat},
                                "lon": {lon}
                            }}
                        }},
                        "scale": "{scale}km",
                        "offset": "{offset}km",
                        "decay": {decay}
                    }}
                }}
            }}
        }}"#,
        lat = coord.lat,
        lon = coord.lon,
        func = decay_fn_param.func,
        scale = decay_fn_param.scale,
        offset = decay_fn_param.offset,
        decay = decay_fn_param.decay
    )
}
