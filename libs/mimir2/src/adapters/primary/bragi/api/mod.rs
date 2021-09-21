use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::adapters::primary::common::coord::Coord;
use crate::adapters::primary::common::filters::Filters;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardGeocoderQuery {
    pub q: String,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub shape: Option<String>,
    pub shape_scope: Option<Vec<String>>, // Here I merge shape and shape_scope together, (and I use str)
    pub datasets: Option<Vec<String>>,
    pub zone_types: Option<Vec<String>>,
    pub poi_types: Option<Vec<String>>,
    pub timeout: u32, // timeout to Elasticsearch in milliseconds
}

impl From<ForwardGeocoderQuery> for Filters {
    fn from(query: ForwardGeocoderQuery) -> Self {
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
// validates input query. In order to do that, ForwardGeocoderQuery must implement
// warp::Reply
#[cfg(test)]
impl warp::Reply for ForwardGeocoderQuery {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::Response::new(serde_json::to_string(&self).unwrap().into())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseGeocoderQuery {
    pub lat: f64,
    pub lon: f64,
    pub timeout: u32, // timeout in milliseconds
}

// For the purpose of testing, we want to be able to test a filter which
// validates input query. In order to do that, ReverseGeocoderQuery must implement
// warp::Reply
#[cfg(test)]
impl warp::Reply for ReverseGeocoderQuery {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::Response::new(serde_json::to_string(&self).unwrap().into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponseBody<D> {
    pub docs: Vec<D>,
    pub docs_count: usize,
}

impl<D> From<Vec<D>> for SearchResponseBody<D> {
    fn from(values: Vec<D>) -> Self {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BragiStatus {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElasticsearchStatus {
    pub version: String,
    pub health: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponseBody {
    pub bragi: BragiStatus,
    pub elasticsearch: ElasticsearchStatus,
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

#[macro_export]
macro_rules! reverse_geocoder {
    ($cl:expr, $st:expr) => {
        routes::reverse_geocoder()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and_then(handlers::reverse_geocoder)
    };
}
pub use reverse_geocoder;

#[macro_export]
macro_rules! status {
    ($cl:expr, $es:expr) => {
        routes::status()
            .and(routes::with_client($cl))
            .and(routes::with_elasticsearch($es))
            .and_then(handlers::status)
    };
}
pub use status;
