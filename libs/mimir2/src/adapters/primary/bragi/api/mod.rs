use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::adapters::primary::common::coord::Coord;
use crate::adapters::primary::common::filters::Filters;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputQuery {
    pub q: String,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub shape: Option<String>,
    pub shape_scope: Option<Vec<String>>, // Here I merge shape and shape_scope together, (and I use str)
    pub datasets: Option<Vec<String>>,
    pub zone_types: Option<Vec<String>>,
    pub poi_types: Option<Vec<String>>,
}

impl From<InputQuery> for Filters {
    fn from(query: InputQuery) -> Self {
        Filters {
            // When option_zip_option becomes available: coord: input.lat.zip_with(input.lon, Coord::new),
            coord: match (query.lat, query.lon) {
                (Some(lat), Some(lon)) => Some(Coord::new(lat, lon)),
                _ => None,
            },
            shape: match (query.shape, query.shape_scope) {
                (Some(shape), Some(shape_scope)) => Some((shape, shape_scope)),
                _ => None,
            },
            datasets: query.datasets,
            zone_types: query.zone_types,
            poi_types: query.poi_types,
        }
    }
}

// For the purpose of testing, we want to be able to test a filter which
// validates input query. In order to do that, InputQuery must implement
// warp::Reply
#[cfg(test)]
impl warp::Reply for InputQuery {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::Response::new(serde_json::to_string(&self).unwrap().into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponseBody {
    pub docs: Vec<JsonValue>,
    pub docs_count: usize,
}

impl From<Vec<JsonValue>> for SearchResponseBody {
    fn from(values: Vec<JsonValue>) -> Self {
        SearchResponseBody {
            docs_count: values.len(),
            docs: values,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResponseBody {
    pub explanation: JsonValue,
}

impl From<JsonValue> for ExplainResponseBody {
    fn from(explanation: JsonValue) -> Self {
        ExplainResponseBody { explanation }
    }
}

#[macro_export]
macro_rules! forward_geocoder {
    ($cl:expr, $st:expr) => {
        routes::forward_geocoder()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and_then(handlers::forward_geocoder)
    };
}
pub use forward_geocoder;
