use std::convert::Infallible;
use warp::{http::StatusCode, path, reject::Reject, Filter, Rejection, Reply};

use crate::adapters::primary::bragi::api::{ForwardGeocoderQuery, ReverseGeocoderQuery};
use crate::adapters::primary::common::settings::QuerySettings;
use crate::domain::ports::primary::search_documents::SearchDocuments;

/// This function defines the base path for Bragi's REST API
fn path_prefix() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    path!("api" / "v1" / ..).boxed()
}

/// This function reads the input parameters on a get request, makes a summary validation
/// of the parameters, and returns them.
pub fn forward_geocoder(
) -> impl Filter<Extract = (ForwardGeocoderQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("autocomplete"))
        .and(forward_geocoder_query())
}

/// This function reads the input parameters on a get request, makes a summary validation
/// of the parameters, and returns them.
pub fn reverse_geocoder(
) -> impl Filter<Extract = (ReverseGeocoderQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("reverse"))
        .and(reverse_geocoder_query())
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

pub fn with_elasticsearch(
    url: String, // elasticsearch url
) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || url.clone())
}

#[derive(Debug)]
struct InvalidRequest;
impl Reject for InvalidRequest {}

pub fn forward_geocoder_query(
) -> impl Filter<Extract = (ForwardGeocoderQuery,), Error = Rejection> + Copy {
    warp::filters::query::query().and_then(|query: ForwardGeocoderQuery| async move {
        // TODO Write actual code to validate the request.
        if query.q.is_empty() {
            Err(warp::reject::custom(InvalidRequest))
        } else {
            Ok(query)
        }
    })
}

pub fn reverse_geocoder_query(
) -> impl Filter<Extract = (ReverseGeocoderQuery,), Error = Rejection> + Copy {
    warp::filters::query::query()
}

pub async fn report_invalid(rejection: Rejection) -> Result<impl Reply, Infallible> {
    let reply = warp::reply::reply();

    if let Some(InvalidRequest) = rejection.find() {
        Ok(warp::reply::with_status(reply, StatusCode::BAD_REQUEST))
    } else {
        // Do better error handling here
        Ok(warp::reply::with_status(
            reply,
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

pub fn status() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::get().and(path_prefix()).and(warp::path("status"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_report_invalid_request_with_empty_query() {
        let filter = forward_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete")
            .reply(&filter);
        assert_eq!(
            resp.await.status(),
            warp::http::status::StatusCode::BAD_REQUEST,
            "Empty query parameter not allowed"
        );
    }

    #[tokio::test]
    async fn should_report_invalid_request_with_invalid_query() {
        let filter = forward_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?place=paris") // place is an unknown key
            .reply(&filter);
        assert_eq!(
            resp.await.status(),
            warp::http::status::StatusCode::BAD_REQUEST,
            "Invalid query parameter not allowed"
        );
    }

    #[tokio::test]
    async fn should_report_valid_request_with_just_query_string() {
        let filter = forward_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/autocomplete?q=paris")
            .reply(&filter)
            .await;
        assert_eq!(
            resp.status(),
            warp::http::status::StatusCode::OK,
            "Expected Status::OK, Got {}: Error Message: {}",
            resp.status(),
            String::from_utf8(resp.body().to_vec()).unwrap()
        );
    }

    #[tokio::test]
    async fn should_report_valid_reverse() {
        let filter = reverse_geocoder();
        let resp = warp::test::request()
            .path("/api/v1/reverse?lat=48.85406&lon=2.33027")
            .reply(&filter)
            .await;
        assert_eq!(
            resp.status(),
            warp::http::status::StatusCode::OK,
            "Expected Status::OK, Got {}: Error Message: {}",
            resp.status(),
            String::from_utf8(resp.body().to_vec()).unwrap()
        );
    }
}
