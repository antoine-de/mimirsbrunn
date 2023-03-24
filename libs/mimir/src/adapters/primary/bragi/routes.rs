use crate::{
    adapters::primary::{
        bragi::{
            api::{
                FeaturesQuery, ForwardGeocoderExplainQuery, ForwardGeocoderQuery, JsonParam,
                ReverseGeocoderQuery, Type,
            },
            handlers::{InternalError, InternalErrorReason},
        },
        common::settings::QuerySettings,
    },
    domain::ports::primary::search_documents::SearchDocuments,
};
use geojson::{GeoJson, Geometry};
use serde::{Deserialize, Serialize};
use serde_qs::Config;
use std::{convert::Infallible, time::Duration};
use tracing::instrument;
use url::Url;
use warp::{
    http::StatusCode,
    path,
    reject::{MethodNotAllowed, Reject},
    Filter, Rejection, Reply,
};

/// This function defines the base path for Bragi's REST API
fn path_prefix() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    path!("api" / "v1" / ..).boxed()
}

/// This is the entry warp filter for the GET autocomplete endpoint
///
/// It validates:
/// * It is a GET HTTP request
/// * The path is <prefix> / autocomplete
/// * It has valid query parameters
///
/// If any of these steps fails, this filter rejects the request
///
/// If all succeed, it returns
/// * a `ForwardGeocoderQuery` structure representing input parameters,
/// * None for the Geometry, since the Geometry can only be obtained from a POST request
#[instrument]
pub fn forward_geocoder_get(
) -> impl Filter<Extract = (ForwardGeocoderQuery, Option<Geometry>), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("autocomplete"))
        .and(warp::path::end())
        .and(forward_geocoder_query()) // We get the query parameters
        .and(warp::any().map(move || None)) // And the shape is None
}

/// This is the entry warp filter for the POST autocomplete endpoint
/// It validates:
/// * It is a POST HTTP request
/// * The path is prefix / autocomplete
/// * It has valid query parameters and the body of the request is a valid shape.
///
/// If any of these steps fails, this filter rejects the request
#[instrument]
pub fn forward_geocoder_post(
) -> impl Filter<Extract = (ForwardGeocoderQuery, Option<Geometry>), Error = Rejection> + Clone {
    warp::post()
        .and(path_prefix())
        .and(warp::path("autocomplete"))
        .and(warp::path::end())
        .and(forward_geocoder_query()) // Query Parameters
        .and(forward_geocoder_body()) // Shape
}

#[instrument]
pub fn forward_geocoder_explain_get(
) -> impl Filter<Extract = (ForwardGeocoderExplainQuery, Option<Geometry>), Error = Rejection> + Clone
{
    warp::get()
        .and(path_prefix())
        .and(warp::path("autocomplete-explain"))
        .and(warp::path::end())
        .and(forward_geocoder_explain_query()) // We get the query parameters
        .and(warp::any().map(move || None)) // And the shape is None
}

#[instrument]
pub fn forward_geocoder_explain_post(
) -> impl Filter<Extract = (ForwardGeocoderExplainQuery, Option<Geometry>), Error = Rejection> + Clone
{
    warp::post()
        .and(path_prefix())
        .and(warp::path("autocomplete-explain"))
        .and(warp::path::end())
        .and(forward_geocoder_explain_query()) // Query Parameters
        .and(forward_geocoder_body()) // Shape
}

/// This function reads the input parameters on a get request, makes a summary validation
/// of the parameters, and returns them.
#[instrument]
pub fn reverse_geocoder(
) -> impl Filter<Extract = (ReverseGeocoderQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("reverse"))
        .and(reverse_geocoder_query())
}

/// This function reads the input parameters on a get request, makes a summary validation
/// of the parameters, and returns them.
#[instrument]
pub fn features() -> impl Filter<Extract = (String, FeaturesQuery), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path!("features" / String))
        .and(features_query())
}

pub fn with_client<S>(s: S) -> impl Filter<Extract = (S,), Error = std::convert::Infallible> + Clone
where
    S: SearchDocuments + Send + Sync + Clone,
{
    warp::any().map(move || s.clone())
}

pub fn with_settings(
    settings: QuerySettings,
) -> impl Filter<Extract = (QuerySettings,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || settings.clone())
}

pub fn with_timeout(
    timeout: Duration,
) -> impl Filter<Extract = (Duration,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || timeout)
}

pub fn with_elasticsearch(
    url: &Url, // elasticsearch url
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    let url = url.to_string();
    warp::any().map(move || url.clone())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiError {
    pub short: String,
    pub long: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum InvalidRequestReason {
    CannotDeserialize,
    EmptyQueryString,
    InconsistentPoiRequest,
    InconsistentZoneRequest,
    InconsistentLatLonRequest,
    OutOfRangeLatLonRequest,
}

#[derive(Deserialize, Serialize, Debug)]
struct InvalidRequest {
    pub reason: InvalidRequestReason,
    pub info: String,
}

impl Reject for InvalidRequest {}

#[derive(Debug)]
struct InvalidPostBody;
impl Reject for InvalidPostBody {}

/// Extract and Validate input parameters from the query
#[instrument]
pub fn forward_geocoder_query(
) -> impl Filter<Extract = (ForwardGeocoderQuery,), Error = Rejection> + Copy {
    warp::filters::query::raw()
        .and_then(|param: String| async move {
            // max_depth=1:
            // for more informations: https://docs.rs/serde_qs/latest/serde_qs/index.html
            let config = Config::new(2, false);
            tracing::info!("Autocomplete query : {}", param);
            config.deserialize_str(&param).map_err(|err| {
                warp::reject::custom(InvalidRequest {
                    reason: InvalidRequestReason::CannotDeserialize,
                    info: err.to_string(),
                })
            })
        })
        .and_then(ensure_query_string_not_empty)
        .and_then(ensure_zone_type_consistent)
        .and_then(ensure_lat_lon_consistent)
}

/// Extract and Validate input parameters from the query
#[instrument]
pub fn forward_geocoder_explain_query(
) -> impl Filter<Extract = (ForwardGeocoderExplainQuery,), Error = Rejection> + Copy {
    warp::filters::query::raw().and_then(|param: String| async move {
        // max_depth=1:
        // for more informations: https://docs.rs/serde_qs/latest/serde_qs/index.html
        let config = Config::new(2, false);
        tracing::info!("forward_geocoder_explain query : {}", param);
        config.deserialize_str(&param).map_err(|err| {
            warp::reject::custom(InvalidRequest {
                reason: InvalidRequestReason::CannotDeserialize,
                info: err.to_string(),
            })
        })
    })
}

pub async fn ensure_query_string_not_empty(
    params: ForwardGeocoderQuery,
) -> Result<ForwardGeocoderQuery, Rejection> {
    if params.q.is_empty() {
        Err(warp::reject::custom(InvalidRequest {
            reason: InvalidRequestReason::EmptyQueryString,
            info: "You must provide and non-empty query string".to_string(),
        }))
    } else {
        Ok(params)
    }
}

/// This filter ensures that if the user specifies lat or lon,
/// then he must specify also lon or lat.
pub async fn ensure_lat_lon_consistent(
    params: ForwardGeocoderQuery,
) -> Result<ForwardGeocoderQuery, Rejection> {
    match (params.lat, params.lon) {
        (Some(lat), Some(lon)) => {
            if !(-90f32..=90f32).contains(&lat) {
                Err(warp::reject::custom(InvalidRequest {
                    reason: InvalidRequestReason::OutOfRangeLatLonRequest,
                    info: format!("requested latitude {} is outside of range [-90;90]", lat),
                }))
            } else if !(-180f32..=180f32).contains(&lon) {
                Err(warp::reject::custom(InvalidRequest {
                    reason: InvalidRequestReason::OutOfRangeLatLonRequest,
                    info: format!("requested longitude {} is outside of range [-180;180]", lon),
                }))
            } else {
                Ok(params)
            }
        }
        (None, None) => Ok(params),
        (_, _) => Err(warp::reject::custom(InvalidRequest {
            reason: InvalidRequestReason::InconsistentLatLonRequest,
            info: "you should provide a 'lon' AND a 'lat' parameter if you provide one of them"
                .to_string(),
        })),
    }
}

/// This filter ensures that if the user requests 'zone', then he must specify the list
/// of zone_types.
pub async fn ensure_zone_type_consistent(
    params: ForwardGeocoderQuery,
) -> Result<ForwardGeocoderQuery, Rejection> {
    if params
        .types
        .as_ref()
        .map(|types| types.iter().any(|s| *s == Type::Zone))
        .unwrap_or(false)
        && params
            .zone_types
            .as_ref()
            .map(|zone_types| zone_types.is_empty())
            .unwrap_or(true)
    {
        Err(warp::reject::custom(InvalidRequest {
            reason: InvalidRequestReason::InconsistentZoneRequest,
            info: "'zone_type' must be specified when you query with 'type' parameter 'zone'"
                .to_string(),
        }))
    } else {
        Ok(params)
    }
}

// This filter extracts the GeoJson shape from the body of the request
#[instrument]
pub fn forward_geocoder_body(
) -> impl Filter<Extract = (Option<Geometry>,), Error = Rejection> + Copy {
    warp::body::content_length_limit(1024 * 32)
        .and(warp::body::json())
        .and_then(validate_geojson_shape)
}

pub async fn validate_geojson_shape(json: JsonParam) -> Result<Option<Geometry>, Rejection> {
    match json.shape {
        GeoJson::Feature(f) => f
            .geometry
            .ok_or_else(|| warp::reject::custom(InvalidPostBody))
            .map(Some),
        _ => Err(warp::reject::custom(InvalidPostBody)),
    }
}

pub fn reverse_geocoder_query(
) -> impl Filter<Extract = (ReverseGeocoderQuery,), Error = Rejection> + Copy {
    warp::filters::query::raw().and_then(|param: String| async move {
        let config = Config::new(2, false);
        tracing::info!("Reverse geocoder query : {}", param);
        config.deserialize_str(&param).map_err(|err| {
            warp::reject::custom(InvalidRequest {
                reason: InvalidRequestReason::CannotDeserialize,
                info: err.to_string(),
            })
        })
    })
}

pub fn features_query() -> impl Filter<Extract = (FeaturesQuery,), Error = Rejection> + Copy {
    warp::filters::query::raw().and_then(|param: String| async move {
        let config = Config::new(2, false);
        tracing::info!("Features query : {}", param);
        config.deserialize_str(&param).map_err(|err| {
            warp::reject::custom(InvalidRequest {
                reason: InvalidRequestReason::CannotDeserialize,
                info: err.to_string(),
            })
        })
    })
}

pub fn status() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::get().and(path_prefix()).and(warp::path("status"))
}

pub fn metrics() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("metrics"))
        .and(warp::path::end())
}

pub async fn report_invalid(rejection: Rejection) -> Result<impl Reply, Infallible> {
    let reply = if let Some(err) = rejection.find::<warp::reject::InvalidQuery>() {
        tracing::warn!("Invalid query {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "invalid query".to_string(),
                long: err.to_string(),
            }),
            StatusCode::BAD_REQUEST,
        )
    } else if let Some(err) = rejection.find::<InvalidRequest>() {
        tracing::warn!("Invalid request {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "validation error".to_string(),
                long: err.info.clone(),
            }),
            StatusCode::BAD_REQUEST,
        )
    } else if let Some(err) = rejection.find::<InternalError>() {
        tracing::warn!("Internal error {:?}", err);
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
        tracing::warn!("MethodNotAllowed {:?}", err);
        warp::reply::with_status(
            warp::reply::json(&ApiError {
                short: "no route".to_string(),
                long: err.to_string(),
            }),
            StatusCode::NOT_FOUND,
        )
    } else {
        tracing::warn!("Internal server error");
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

#[cfg(test)]
mod tests {
    use super::*;
    use places::PlaceDocType;

    #[tokio::test]
    async fn should_report_invalid_query_with_no_query() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete")
            .filter(&filter)
            .await;
        assert!(
            resp.unwrap_err()
                .find::<warp::reject::InvalidQuery>()
                .is_some(),
            "Empty query parameter not allowed"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_empty_query_string() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=")
            .filter(&filter)
            .await;
        assert_eq!(
            resp.unwrap_err().find::<InvalidRequest>().unwrap().reason,
            InvalidRequestReason::EmptyQueryString,
            "Empty query string not allowed"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_invalid_query() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?place=paris") // place is an unknown key
            .filter(&filter)
            .await;
        assert_eq!(
            resp.unwrap_err().find::<InvalidRequest>().unwrap().reason,
            InvalidRequestReason::CannotDeserialize,
            "Unknown parameter, cannot deserialize"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_invalid_type() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?place=paris&type[]=country") // place is an unknown key
            .filter(&filter)
            .await;
        assert_eq!(
            resp.unwrap_err().find::<InvalidRequest>().unwrap().reason,
            InvalidRequestReason::CannotDeserialize,
            "Unknown type, cannot deserialize"
        );
    }

    #[tokio::test]
    async fn should_correctly_extract_query_string() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=paris")
            .filter(&filter)
            .await;
        assert_eq!(resp.unwrap().0.q, String::from("paris"));
    }

    #[tokio::test]
    async fn should_correctly_extract_types() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=paris&type[]=street&type[]=zone&zone_type[]=city")
            .filter(&filter)
            .await;
        assert_eq!(resp.as_ref().unwrap().0.types.as_ref().unwrap().len(), 2);
        assert!(resp
            .unwrap()
            .0
            .types
            .unwrap()
            .iter()
            .zip([Type::Street, Type::Zone].iter())
            .all(|(a, b)| *a == *b));
    }

    // TODO The shape_scope parameter can only be used with a POST request (since that's the only
    // way of specifying the shape). But to write a test for that case, we'd need to have access
    // to both the query parameters (ForwardGeocoderQuery) and the body (Option<Geometry>) which
    // is possible at the handler level...

    #[tokio::test]
    async fn should_correctly_extract_geojson_shape() {
        let filter = forward_geocoder_post();
        let resp = warp::test::request()
            .method("POST")
            .path("/api/v1/autocomplete?q=paris")
            .body(r#"{"shape":{"type":"Feature","properties":{},"geometry":{"type":"Polygon", "coordinates":[[[2.376488, 48.846431],
        [2.376306, 48.846430],[2.376309, 48.846606],[2.376486, 48.846603], [2.376488, 48.846431]]]}}}"#)
            .filter(&filter)
            .await;
        assert!(resp.unwrap().1.is_some());
    }

    #[tokio::test]
    async fn should_report_invalid_shape() {
        let filter = forward_geocoder_post();
        let resp = warp::test::request()
            .method("POST")
            .path("/api/v1/autocomplete?q=paris")
            .body(r#"{"shape":{"type":"Feature","properties":{}}}"#)
            .filter(&filter)
            .await;
        println!("{:?}", resp);
        assert!(
            resp.unwrap_err()
                .find::<warp::filters::body::BodyDeserializeError>()
                .unwrap()
                .to_string()
                .contains("Expected a GeoJSON property for `geometry`"),
            "Invalid GeoJSON shape (missing geometry). cannot deserialize body"
        );
    }

    #[tokio::test]
    async fn should_correctly_extract_query_no_strict_mode() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&type%5B%5D=street&type%5B%5D=house")
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(resp.0.types.unwrap(), [Type::Street, Type::House]);
        assert_eq!(resp.0.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_pt_dataset() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&pt_dataset[]=dataset1&pt_dataset[]=dataset2")
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(resp.0.pt_dataset.unwrap(), ["dataset1", "dataset2"]);
        assert_eq!(resp.0.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_request_id() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&request_id=xxxx-yyyyy-zzzz")
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(resp.0.request_id.unwrap(), "xxxx-yyyyy-zzzz");
        assert_eq!(resp.0.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_poi_dataset() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path(
                "/api/v1/autocomplete?q=Bob&poi_dataset[]=poi-dataset1&poi_dataset[]=poi-dataset2",
            )
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(
            resp.0.poi_dataset.unwrap(),
            ["poi-dataset1", "poi-dataset2"]
        );
        assert_eq!(resp.0.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_shape_scope() {
        let filter = forward_geocoder_get();
        let resp = warp::test::request()
            .path(
                "/api/v1/autocomplete?q=Bob&shape_scope[]=admin&shape_scope[]=street\
                &shape_scope[]=addr&shape_scope[]=poi&type%5B%5D=house&shape_scope[]=stop",
            )
            .filter(&filter)
            .await
            .unwrap();

        assert_eq!(
            resp.0.shape_scope.unwrap(),
            [
                PlaceDocType::Admin,
                PlaceDocType::Street,
                PlaceDocType::Addr,
                PlaceDocType::Poi,
                PlaceDocType::Stop
            ]
        );
        assert_eq!(resp.0.types.unwrap(), [Type::House]);
        assert_eq!(resp.0.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_default_limit() {
        let filter = reverse_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/reverse?lon=6.15&lat=49.14")
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(resp.limit, 1);
    }

    #[tokio::test]
    async fn should_correctly_extract_with_limit() {
        let filter = reverse_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/reverse?lon=6.15&lat=49.14&limit=20")
            .filter(&filter)
            .await
            .unwrap();
        assert_eq!(resp.limit, 20);
    }
}
