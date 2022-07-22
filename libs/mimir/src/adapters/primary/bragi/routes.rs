use crate::adapters::primary::bragi::api::{ForwardGeocoderQuery, Type};
use crate::adapters::primary::bragi::handlers::{InternalError, InternalErrorReason};
use geojson::{GeoJson, Geometry};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_qs::Config;
use std::convert::Infallible;
use tracing::instrument;
use warp::http::StatusCode;
use warp::reject::{MethodNotAllowed, Reject};
use warp::{Filter, Rejection, Reply};

use super::api::ForwardGeocoderBody;

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiError {
    pub short: String,
    pub long: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum InvalidRequestReason {
    CannotDeserialize,
    EmptyQueryString,
    InconsistentPoiRequest,
    InconsistentZoneRequest,
    InconsistentLatLonRequest,
    OutOfRangeLatLonRequest,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InvalidRequest {
    pub reason: InvalidRequestReason,
    pub info: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ValidationError(pub &'static str);

impl Reject for ValidationError {}
impl Reject for InvalidRequest {}

#[derive(Debug)]
struct InvalidPostBody;
impl Reject for InvalidPostBody {}

/// Helper validation function that will extract query parameters into a struct.
pub fn validate_query_params<T>() -> impl Filter<Extract = (T,), Error = Rejection> + Copy
where
    T: DeserializeOwned + Send + Sync,
{
    warp::filters::query::raw().and_then(|param: String| async move {
        // max_depth=1: for more informations: https://docs.rs/serde_qs/latest/serde_qs/index.html
        let config = Config::new(2, false);
        tracing::info!("Query params: {}", param);

        config.deserialize_str(&param).map_err(|err| {
            warp::reject::custom(InvalidRequest {
                reason: InvalidRequestReason::CannotDeserialize,
                info: err.to_string(),
            })
        })
    })
}

/// This filter ensures that if the user requests 'zone', then he must specify the list
/// of zone_types.
pub fn is_valid_zone_type(params: &ForwardGeocoderQuery) -> bool {
    params
        .types
        .as_ref()
        .map(|types| types.iter().all(|s| *s != Type::Zone))
        .unwrap_or(true)
        || params
            .zone_types
            .as_ref()
            .map(|zone_types| !zone_types.is_empty())
            .unwrap_or(false)
}

// This filter extracts the GeoJson shape from the body of the request
#[instrument]
pub fn validate_geojson_body(
) -> impl Filter<Extract = (Option<Geometry>,), Error = Rejection> + Copy {
    warp::body::content_length_limit(1024 * 32)
        .and(warp::body::json())
        .and_then(|json: ForwardGeocoderBody| async move {
            match json.shape {
                GeoJson::Feature(f) => f
                    .geometry
                    .ok_or_else(|| warp::reject::custom(InvalidPostBody))
                    .map(Some),
                _ => Err(warp::reject::custom(InvalidPostBody)),
            }
        })
}

pub async fn report_invalid(rejection: Rejection) -> Result<impl Reply, Infallible> {
    let reply = if let Some(err) = rejection.find::<warp::reject::InvalidQuery>() {
        tracing::info!("Invalid query {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "invalid query".to_string(),
                long: err.to_string(),
            }),
            StatusCode::BAD_REQUEST,
        )
    } else if let Some(err) = rejection.find::<InvalidRequest>() {
        tracing::info!("Invalid request {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "validation error".to_string(),
                long: err.info.clone(),
            }),
            StatusCode::BAD_REQUEST,
        )
    } else if let Some(err) = rejection.find::<InternalError>() {
        tracing::info!("Internal error {:?}", err);
        let short = match err.reason {
            InternalErrorReason::ObjectNotFoundError => "Unable to find object".to_string(),
            _ => "query error".to_string(),
        };
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short,
                long: err.info.clone(),
            }),
            StatusCode::BAD_REQUEST,
        )
    } else if let Some(err) = rejection.find::<MethodNotAllowed>() {
        tracing::info!("MethodNotAllowed {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "no route".to_string(),
                long: err.to_string(),
            }),
            StatusCode::NOT_FOUND,
        )
    } else {
        tracing::info!("Internal server error");
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "INTERNAL_SERVER_ERROR".to_string(),
                long: "INTERNAL_SERVER_ERROR".to_string(),
            }),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    };
    let reply = warp::reply::with_header(reply, "content-type", "application/json");
    Ok(reply)
}

pub fn cache_filter<F, T>(
    filter: F,
    http_cache_duration: usize,
) -> impl Filter<Extract = impl Reply, Error = std::convert::Infallible> + Clone + Send + Sync
where
    F: Filter<Extract = (T,), Error = std::convert::Infallible> + Clone + Send + Sync,
    F::Extract: warp::Reply,
    T: warp::Reply,
{
    warp::any().and(filter).map(move |reply| {
        warp::reply::with_header(
            reply,
            "cache-control",
            format!("max-age={}", http_cache_duration),
        )
    })
}
