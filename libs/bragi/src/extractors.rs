/// [Actix extractors](https://actix.rs/docs/extractors/) used to have
/// a coherent error handling for all apis
///
/// All Bragi's api should use them instead of the default Actix's Query extractors
/// We don't need a custom Path since the error handling for missing a Path extractor is to get a 404
///
/// Note: we use serde_qs instead of the actix's default serde_urlencoded because serde_qs is more flexible
/// (cf https://github.com/nox/serde_urlencoded/issues/6)
use crate::model::ApiError;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use std::ops::{Deref, DerefMut};

#[derive(Fail, Debug)]
pub enum ActixError {
    #[fail(display = "invalid json: {}", _0)]
    InvalidJson(String), //TODO: error instead of string ?
    #[fail(display = "invalid argument: {}", _0)]
    InvalidQueryParam(String),
    #[fail(display = "route '{}' does not exists", _0)]
    RouteNotFound(String),
}

impl actix_web::error::ResponseError for ActixError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match *self {
            ActixError::InvalidJson(_) => actix_web::HttpResponse::BadRequest().json(ApiError {
                short: "validation error".to_owned(),
                long: format!("{}", self),
            }),
            ActixError::InvalidQueryParam(_) => {
                actix_web::HttpResponse::BadRequest().json(ApiError {
                    short: "validation error".to_owned(),
                    long: format!("{}", self),
                })
            }
            ActixError::RouteNotFound(_) => actix_web::HttpResponse::NotFound().json(ApiError {
                short: "no route".to_owned(),
                long: format!("{}", self),
            }),
        }
    }
}

pub struct BragiQuery<T>(T);

impl<T> Deref for BragiQuery<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for BragiQuery<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T, S> FromRequest<S> for BragiQuery<T>
where
    T: serde::de::DeserializeOwned,
{
    type Config = actix_web::dev::QueryConfig<S>;
    type Result = Result<Self, ActixError>;

    #[inline]
    fn from_request(req: &HttpRequest<S>, _cfg: &Self::Config) -> Self::Result {
        // Note: we need a non strict serde_qs to be able to parse the %5B / %5D as '[' / ']'
        serde_qs::Config::new(5, false)
            .deserialize_str(req.query_string())
            .map_err(|e| ActixError::InvalidQueryParam(format!("{}", e)))
            .map(BragiQuery)
    }
}
