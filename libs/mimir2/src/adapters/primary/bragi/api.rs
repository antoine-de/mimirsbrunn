use cosmogony::ZoneType;
use geojson::{GeoJson, Geometry};
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
    pub shape_scope: Option<Vec<Type>>,
    pub datasets: Option<Vec<String>>,
    #[serde(default, rename = "type")]
    pub types: Option<Vec<Type>>,
    #[serde(default, rename = "zone_type")]
    pub zone_types: Option<Vec<ZoneType>>,
    pub poi_types: Option<Vec<String>>,
}

impl From<(ForwardGeocoderQuery, Option<Geometry>)> for Filters {
    fn from(query: (ForwardGeocoderQuery, Option<Geometry>)) -> Self {
        Filters {
            // When option_zip_option becomes available: coord: input.lat.zip_with(input.lon, Coord::new),
            coord: match (query.0.lat, query.0.lon) {
                (Some(lat), Some(lon)) => Some(Coord::new(lat, lon)),
                _ => None,
            },
            shape: None, // Not implemented yet.... soon!
            datasets: query.0.datasets,
            zone_types: query.0.zone_types.map(|zts| {
                zts.iter()
                    .map(|zt| serde_json::to_string(zt).unwrap())
                    .collect()
            }),
            poi_types: query.0.poi_types,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseGeocoderQuery {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonParam {
    pub shape: GeoJson,
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
}
