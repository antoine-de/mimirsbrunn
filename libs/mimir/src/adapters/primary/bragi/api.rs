use crate::utils::deserialize::deserialize_opt_duration;
use cosmogony::ZoneType;
use geojson::{GeoJson, Geometry};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;

use crate::adapters::primary::common::{coord::Coord, filters::Filters};
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, PlaceDocType};

pub const DEFAULT_LIMIT_RESULT_ES: i64 = 10;
pub const DEFAULT_LIMIT_RESULT_REVERSE_API: i64 = 1;
pub const DEFAULT_LANG: &str = "fr";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForwardGeocoderExplainQuery {
    pub doc_id: String,
    pub doc_type: String,

    // Fields from ForwardGeocoderQuery are repeated here as nesting two levels
    // of `flatten` with serde_qs is not supported.
    // See https://github.com/samscott89/serde_qs/issues/14
    pub q: String,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub shape_scope: Option<Vec<PlaceDocType>>,
    #[serde(default, rename = "type")]
    pub types: Option<Vec<Type>>,
    #[serde(default, rename = "zone_type")]
    pub zone_types: Option<Vec<ZoneType>>,
    pub poi_types: Option<Vec<String>>,
    #[serde(default = "default_result_limit")]
    pub limit: i64,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(deserialize_with = "deserialize_opt_duration", default)]
    pub timeout: Option<Duration>,
    pub pt_dataset: Option<Vec<String>>,
    pub poi_dataset: Option<Vec<String>>,
    pub request_id: Option<String>,
    #[serde(flatten)]
    pub proximity: Option<Proximity>,
}

impl From<ForwardGeocoderExplainQuery> for ForwardGeocoderQuery {
    fn from(val: ForwardGeocoderExplainQuery) -> Self {
        let ForwardGeocoderExplainQuery {
            q,
            lat,
            lon,
            shape_scope,
            types,
            zone_types,
            poi_types,
            limit,
            lang,
            timeout,
            pt_dataset,
            poi_dataset,
            request_id,
            proximity,
            ..
        } = val;

        ForwardGeocoderQuery {
            q,
            lat,
            lon,
            shape_scope,
            types,
            zone_types,
            poi_types,
            limit,
            lang,
            timeout,
            pt_dataset,
            poi_dataset,
            request_id,
            proximity,
        }
    }
}

/// This structure contains all the query parameters that
/// can be submitted for the autocomplete endpoint.
///
/// Only the `q` parameter is mandatory.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForwardGeocoderQuery {
    pub q: String,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub shape_scope: Option<Vec<PlaceDocType>>,
    #[serde(default, rename = "type")]
    pub types: Option<Vec<Type>>,
    #[serde(default, rename = "zone_type")]
    pub zone_types: Option<Vec<ZoneType>>,
    pub poi_types: Option<Vec<String>>,
    #[serde(default = "default_result_limit")]
    pub limit: i64,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(deserialize_with = "deserialize_opt_duration", default)]
    pub timeout: Option<Duration>,
    pub pt_dataset: Option<Vec<String>>,
    pub poi_dataset: Option<Vec<String>>,
    pub request_id: Option<String>,
    #[serde(flatten)]
    pub proximity: Option<Proximity>,
}

fn default_result_limit() -> i64 {
    DEFAULT_LIMIT_RESULT_ES
}

fn default_result_limit_reverse() -> i64 {
    DEFAULT_LIMIT_RESULT_REVERSE_API
}

fn default_lang() -> String {
    DEFAULT_LANG.to_string()
}

impl From<(ForwardGeocoderQuery, Option<Geometry>)> for Filters {
    fn from(source: (ForwardGeocoderQuery, Option<Geometry>)) -> Self {
        let (query, geometry) = source;
        let zone_types = query
            .zone_types
            .map(|zts| zts.iter().map(|t| t.as_str().to_string()).collect());
        Filters {
            // When option_zip_option becomes available: coord: input.lat.zip_with(input.lon, Coord::new),
            coord: match (query.lat, query.lon) {
                (Some(lat), Some(lon)) => Some(Coord::new(lat, lon)),
                _ => None,
            },
            shape: geometry.map(|geometry| {
                (
                    geometry,
                    query.shape_scope.map_or_else(
                        || {
                            vec![
                                PlaceDocType::Poi,
                                PlaceDocType::Street,
                                PlaceDocType::Admin,
                                PlaceDocType::Addr,
                                PlaceDocType::Stop,
                            ]
                            .iter()
                            .map(|t| t.as_str().to_string())
                            .collect()
                        },
                        |shape_scope| shape_scope.iter().map(|t| t.as_str().to_string()).collect(),
                    ),
                )
            }),
            zone_types,
            poi_types: query.poi_types,
            limit: query.limit,
            timeout: query.timeout,
            proximity: query.proximity,
        }
    }
}

/// This structure contains all the query parameters that
/// can be submitted for the features endpoint.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FeaturesQuery {
    pub pt_dataset: Option<Vec<String>>,
    pub poi_dataset: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_opt_duration", default)]
    pub timeout: Option<Duration>,
}

/// This structure contains all the query parameters that
/// can be submitted for the reverse endpoint.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReverseGeocoderQuery {
    pub lat: f64,
    pub lon: f64,
    #[serde(default = "default_result_limit_reverse")]
    pub limit: i64,
    #[serde(deserialize_with = "deserialize_opt_duration", default)]
    pub timeout: Option<Duration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonParam {
    pub shape: GeoJson,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExplainResponseBody {
    pub explanation: JsonValue,
}

impl From<JsonValue> for ExplainResponseBody {
    fn from(explanation: JsonValue) -> Self {
        ExplainResponseBody { explanation }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BragiStatus {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MimirStatus {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ElasticsearchStatus {
    pub version: String,
    pub health: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StatusResponseBody {
    pub bragi: BragiStatus,
    pub mimir: MimirStatus,
    pub elasticsearch: ElasticsearchStatus,
}

/// This macro is used to define the `forward_geocoder` route.
/// It takes a client (`ElasticsearchStorage`) and query settings
/// It can be either a GET request, with query parameters,
/// or a POST request, with both query parameters and a GeoJson shape
/// in the body.
#[macro_export]
macro_rules! forward_geocoder {
    ($cl:expr, $st:expr, $ti:expr) => {
        routes::forward_geocoder_get()
            .or(routes::forward_geocoder_post())
            .unify()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and(routes::with_timeout($ti))
            .and_then(handlers::forward_geocoder)
    };
}

pub use forward_geocoder;

#[macro_export]
macro_rules! forward_geocoder_explain {
    ($cl:expr, $st:expr, $ti:expr) => {
        routes::forward_geocoder_explain_get()
            .or(routes::forward_geocoder_explain_post())
            .unify()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and(routes::with_timeout($ti))
            .and_then(handlers::forward_geocoder_explain)
    };
}
pub use forward_geocoder_explain;

#[macro_export]
macro_rules! reverse_geocoder {
    ($cl:expr, $st:expr, $ti:expr) => {
        routes::reverse_geocoder()
            .and(routes::with_client($cl))
            .and(routes::with_settings($st))
            .and(routes::with_timeout($ti))
            .and_then(handlers::reverse_geocoder)
    };
}
pub use reverse_geocoder;

#[macro_export]
macro_rules! features {
    ($cl:expr, $ti:expr) => {
        routes::features()
            .and(routes::with_client($cl))
            .and(routes::with_timeout($ti))
            .and_then(handlers::features)
    };
}
pub use features;

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

#[macro_export]
macro_rules! metrics {
    () => {
        routes::metrics().and_then(handlers::metrics)
    };
}
pub use metrics;

#[derive(PartialEq, Eq, Copy, Clone, Debug, Deserialize, Serialize)]
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
    // TODO To be deleted when switching to full ES7 (in production)
    #[serde(rename = "city")]
    City,
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        match self {
            Type::House => "house",
            Type::Poi => "poi",
            Type::StopArea => "public_transport:stop_area",
            Type::Street => "street",
            Type::Zone => "zone",
            Type::City => "city",
        }
    }

    pub fn as_index_type(&self) -> &'static str {
        match self {
            Type::House => Addr::static_doc_type(),
            Type::Poi => Poi::static_doc_type(),
            Type::StopArea => Stop::static_doc_type(),
            Type::Street => Street::static_doc_type(),
            Type::Zone | Type::City => Admin::static_doc_type(),
        }
    }
}

#[serde_with::serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Proximity {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "proximity_scale")]
    pub scale: f64,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "proximity_offset")]
    pub offset: f64,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[serde(rename = "proximity_decay")]
    pub decay: f64,
}
