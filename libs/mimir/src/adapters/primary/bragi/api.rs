use crate::utils::deserialize::deserialize_opt_duration;
use cosmogony::ZoneType;
use geojson::{GeoJson, Geometry};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;

use crate::adapters::primary::common::coord::Coord;
use crate::adapters::primary::common::filters::Filters;
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};

pub const DEFAULT_LIMIT_RESULT_ES: i64 = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardGeocoderExplainQuery {
    pub doc_id: String,
    pub doc_type: String,

    #[serde(flatten)]
    pub query: ForwardGeocoderQuery,
}

/// This structure contains all the query parameters that
/// can be submitted for the autocomplete endpoint.
///
/// Only the `q` parameter is mandatory.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardGeocoderQuery {
    pub q: String,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub shape_scope: Option<Vec<Type>>,
    pub datasets: Option<Vec<String>>,
    #[serde(default, rename = "type")]
    pub types: Option<Vec<Type>>,
    #[serde(default, rename = "zone_type")]
    pub zone_types: Option<Vec<ZoneType>>,
    pub poi_types: Option<Vec<String>>,
    #[serde(default = "default_result_limit")]
    pub limit: i64,
    #[serde(deserialize_with = "deserialize_opt_duration", default)]
    pub timeout: Option<Duration>,
}

fn default_result_limit() -> i64 {
    DEFAULT_LIMIT_RESULT_ES
}

impl From<(ForwardGeocoderQuery, Option<Geometry>)> for Filters {
    fn from(source: (ForwardGeocoderQuery, Option<Geometry>)) -> Self {
        let (
            ForwardGeocoderQuery {
                q: _,
                lat,
                lon,
                shape_scope,
                datasets,
                types: _,
                zone_types,
                poi_types,
                limit,
                timeout,
            },
            geometry,
        ) = source;
        let zone_types = zone_types.map(|zts| {
            zts.iter()
                .map(|t| t.as_str().to_string())
                .collect()
        });
        Filters {
            // When option_zip_option becomes available: coord: input.lat.zip_with(input.lon, Coord::new),
            coord: match (lat, lon) {
                (Some(lat), Some(lon)) => Some(Coord::new(lat, lon)),
                _ => None,
            },
            shape: geometry.map(|geometry| {
                (
                    geometry,
                    shape_scope
                        .map(|shape_scope| {
                            shape_scope.iter().map(|t| t.as_str().to_string()).collect()
                        })
                        .unwrap_or_else(|| {
                            vec![
                                Type::House,
                                Type::Poi,
                                Type::StopArea,
                                Type::Street,
                                Type::Zone,
                            ]
                            .iter()
                            .map(|t| t.as_str().to_string())
                            .collect()
                        }),
                )
            }),
            datasets,
            zone_types,
            poi_types,
            limit,
            timeout,
        }
    }
}

/// This structure contains all the query parameters that
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseGeocoderQuery {
    pub lat: f64,
    pub lon: f64,
    #[serde(default = "default_result_limit")]
    pub limit: i64,
    #[serde(deserialize_with = "deserialize_opt_duration")]
    pub timeout: Option<Duration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonParam {
    pub shape: GeoJson,
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
pub struct MimirStatus {
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
    pub mimir: MimirStatus,
    pub elasticsearch: ElasticsearchStatus,
}

/// This macro is used to define the forward_geocoder route.
/// It takes a client (ElasticsearchStorage) and query settings
/// It can be either a GET request, with query parameters,
/// or a POST request, with both query parameters and a GeoJson shape
/// in the body.
#[macro_export]
macro_rules! forward_geocoder {
    ($cl:expr, $st:expr) => {
        routes::forward_geocoder_get()
            .or(routes::forward_geocoder_post())
            .unify()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and_then(handlers::forward_geocoder)
    };
}

pub use forward_geocoder;

#[macro_export]
macro_rules! forward_geocoder_explain {
    ($cl:expr, $st:expr) => {
        routes::forward_geocoder_explain_get()
            .or(routes::forward_geocoder_explain_post())
            .unify()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and_then(handlers::forward_geocoder_explain)
    };
}
pub use forward_geocoder_explain;

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

#[derive(PartialEq, Copy, Clone, Debug, Deserialize, Serialize)]
pub enum Type {
    #[serde(rename = "house")]
    House,
    #[serde(rename = "poi")]
    Poi,
    #[serde(rename = "public_transport:stop_area")]
    StopArea,
    #[serde(rename = "street")]
    Street,
    #[serde(rename = "zone")]
    Zone,
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        match self {
            Type::House => "house",
            Type::Poi => "poi",
            Type::StopArea => "public_transport:stop_area",
            Type::Street => "street",
            Type::Zone => "zone",
        }
    }

    pub fn as_index_type(&self) -> &'static str {
        match self {
            Type::House => Addr::static_doc_type(),
            Type::Poi => Poi::static_doc_type(),
            Type::StopArea => Stop::static_doc_type(),
            Type::Street => Street::static_doc_type(),
            Type::Zone => Admin::static_doc_type(),
        }
    }
}
