use std::convert::Infallible;
use warp::{http::StatusCode, path, reject::Reject, Filter, Rejection, Reply};

use crate::adapters::primary::bragi::api::InputQuery;
use crate::adapters::primary::common::settings::QuerySettings;
use crate::domain::ports::primary::search_documents::SearchDocuments;

/// This function defines the base path for Bragi's REST API
fn path_prefix() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    path!("api" / "v1" / ..).boxed()
}

/// This function reads the input parameters on a get request, makes a summary validation
/// of the parameters, and returns them.
pub fn forward_geocoder() -> impl Filter<Extract = (InputQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("autocomplete"))
        .and(forward_geocoder_query())
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

#[derive(Debug)]
struct InvalidRequest;
impl Reject for InvalidRequest {}

pub fn forward_geocoder_query() -> impl Filter<Extract = (InputQuery,), Error = Rejection> + Copy {
    warp::filters::query::query().and_then(|query: InputQuery| async move {
        // TODO Write actual code to validate the request.
        if query.q.is_empty() {
            Err(warp::reject::custom(InvalidRequest))
        } else {
            Ok(query)
        }
    })
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
            .path("/api/v1/autocomplete?place=paris") // place is unknown?
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
            .reply(&filter);
        assert_eq!(
            resp.await.status(),
            warp::http::status::StatusCode::OK,
            "Valid Query Parameter"
        );
    }
}
