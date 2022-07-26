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

impl Reject for InvalidRequest {}

#[derive(Debug)]
struct InvalidPostBody;
impl Reject for InvalidPostBody {}

pub fn validate_forward_geocoder(
) -> impl Clone + Filter<Extract = (ForwardGeocoderQuery, Option<Geometry>), Error = Rejection> {
    {
        warp::get()
            .and(ForwardGeocoderQuery::validate())
            .and(warp::any().map(|| None)) // the shape is None
    }
    .or({
        warp::post()
            .and(ForwardGeocoderQuery::validate())
            .and(validate_geojson_body())
    })
    .unify()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::primary::bragi::api::ReverseGeocoderQuery;

    #[tokio::test]
    async fn should_report_invalid_query_with_no_query() {
        let resp = warp::test::request()
            .path("/api/v1/autocomplete")
            .filter(&validate_forward_geocoder())
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
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=")
            .filter(&validate_forward_geocoder())
            .await;

        assert_eq!(
            dbg!(resp.unwrap_err())
                .find::<InvalidRequest>()
                .unwrap()
                .reason,
            InvalidRequestReason::EmptyQueryString,
            "Empty query string not allowed"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_invalid_query() {
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?place=paris") // place is an unknown key
            .filter(&validate_forward_geocoder())
            .await;

        assert_eq!(
            resp.unwrap_err().find::<InvalidRequest>().unwrap().reason,
            InvalidRequestReason::CannotDeserialize,
            "Unknown parameter, cannot deserialize"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_invalid_type() {
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?place=paris&type[]=country") // place is an unknown key
            .filter(&validate_forward_geocoder())
            .await;

        assert_eq!(
            resp.unwrap_err().find::<InvalidRequest>().unwrap().reason,
            InvalidRequestReason::CannotDeserialize,
            "Unknown type, cannot deserialize"
        );
    }

    #[tokio::test]
    async fn should_correctly_extract_query_string() {
        let (query, _) = warp::test::request()
            .path("/api/v1/autocomplete?q=paris")
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.q, "paris");
    }

    #[tokio::test]
    async fn should_correctly_extract_types() {
        let (query, _) = warp::test::request()
            .path("/api/v1/autocomplete?q=paris&type[]=street&type[]=zone&zone_type[]=city")
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.types.unwrap(), [Type::Street, Type::Zone]);
    }

    // TODO The shape_scope parameter can only be used with a POST request (since that's the only
    // way of specifying the shape). But to write a test for that case, we'd need to have access
    // to both the query parameters (ForwardGeocoderQuery) and the body (Option<Geometry>) which
    // is possible at the handler level...

    #[tokio::test]
    async fn should_correctly_extract_geojson_shape() {
        let (_, geom) = warp::test::request()
            .method("POST")
            .path("/api/v1/autocomplete?q=paris")
            .body(
                r#"{
                    "shape": {
                        "type":"Feature",
                        "properties": {},
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [[
                                [2.376488, 48.846431],
                                [2.376306, 48.846430],
                                [2.376309, 48.846606],
                                [2.376486, 48.846603],
                                [2.376488, 48.846431]
                            ]]
                        }
                    }
                }"#,
            )
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert!(geom.is_some());
    }

    #[tokio::test]
    async fn should_report_invalid_shape() {
        let resp = warp::test::request()
            .method("POST")
            .path("/api/v1/autocomplete?q=paris")
            .body(r#"{"shape": {"type": "Feature", "properties": {}}}"#)
            .filter(&validate_forward_geocoder())
            .await;

        assert!(
            dbg!(resp.unwrap_err())
                .find::<warp::filters::body::BodyDeserializeError>()
                .unwrap()
                .to_string()
                .contains("Expected a GeoJSON property for `geometry`"),
            "Invalid GeoJSON shape (missing geometry). cannot deserialize body"
        );
    }

    #[tokio::test]
    async fn should_correctly_extract_query_no_strict_mode() {
        let (query, _) = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&type%5B%5D=street&type%5B%5D=house")
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.types.unwrap(), [Type::Street, Type::House]);
        assert_eq!(query.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_pt_dataset() {
        let (query, _) = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&pt_dataset[]=dataset1&pt_dataset[]=dataset2")
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.pt_dataset.unwrap(), ["dataset1", "dataset2"]);
        assert_eq!(query.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_request_id() {
        let (query, _) = warp::test::request()
            .path("/api/v1/autocomplete?q=Bob&request_id=xxxx-yyyyy-zzzz")
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.request_id.unwrap(), "xxxx-yyyyy-zzzz");
        assert_eq!(query.q, "Bob");
    }

    #[tokio::test]
    async fn should_correctly_extract_poi_dataset() {
        let (query, _) = warp::test::request()
            .path(
                "/api/v1/autocomplete?q=Bob&poi_dataset[]=poi-dataset1&poi_dataset[]=poi-dataset2",
            )
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.q, "Bob");
        assert_eq!(query.poi_dataset.unwrap(), ["poi-dataset1", "poi-dataset2"]);
    }

    #[tokio::test]
    async fn should_correctly_extract_shape_scope() {
        let (query, _) = warp::test::request()
            .path(
                "/api/v1/autocomplete?q=Bob&shape_scope[]=admin&shape_scope[]=street\
                &shape_scope[]=addr&shape_scope[]=poi&type%5B%5D=house&shape_scope[]=stop",
            )
            .filter(&validate_forward_geocoder())
            .await
            .unwrap();

        assert_eq!(query.types.unwrap(), [Type::House]);
        assert_eq!(query.q, "Bob");
        assert_eq!(
            query.shape_scope.unwrap(),
            [
                places::PlaceDocType::Admin,
                places::PlaceDocType::Street,
                places::PlaceDocType::Addr,
                places::PlaceDocType::Poi,
                places::PlaceDocType::Stop
            ]
        );
    }

    #[tokio::test]
    async fn should_correctly_extract_default_limit() {
        let resp = warp::test::request()
            .path("/api/v1/reverse?lon=6.15&lat=49.14")
            .filter(&ReverseGeocoderQuery::validate())
            .await
            .unwrap();

        assert_eq!(resp.limit, 1);
    }

    #[tokio::test]
    async fn should_correctly_extract_with_limit() {
        let resp = warp::test::request()
            .path("/api/v1/reverse?lon=6.15&lat=49.14&limit=20")
            .filter(&ReverseGeocoderQuery::validate())
            .await
            .unwrap();

        assert_eq!(resp.limit, 20);
    }
}
