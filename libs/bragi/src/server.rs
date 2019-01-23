use crate::model::ApiError;
use crate::prometheus_middleware;
use crate::routes::{autocomplete, entry_point, post_autocomplete, status, metrics};
use crate::{Args, Context};
use actix_web::{http, middleware, server, App};
use structopt::StructOpt;

#[derive(Fail, Deserialize, Debug)]
enum ActixError {
    #[fail(display = "invalid json: {}", _0)]
    InvalidJson(String), //TODO: error instead of string ?
    #[fail(display = "invalid argument: {}", _0)]
    InvalidQueryParam(String), //TODO: error instead of string ?
}

impl actix_web::error::ResponseError for ActixError {
    fn error_response(&self) -> actix_web::HttpResponse {
        error!("hoooo une erreur actix: {:?}", self);
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
        }
    }
}

pub fn create_server(ctx: Context) -> App<Context> {
    App::with_state(ctx)
        .middleware(
            middleware::cors::Cors::build()
                .allowed_methods(vec!["GET"])
                .finish(),
        )
        .middleware(middleware::Logger::default())
        .middleware(prometheus_middleware::PrometheusMiddleware::default())
        .resource("/", |r| r.f(entry_point))
        .resource("/autocomplete", |r| {
            r.method(http::Method::GET)
                .with_config(autocomplete, |(query_cfg, _)| {
                    query_cfg.error_handler(|err, _req| {
                        ActixError::InvalidQueryParam(format!("{}", err)).into()
                    });
                });
            r.method(http::Method::POST).with_config(
                post_autocomplete,
                |(query_cfg, _, json_cfg)| {
                    json_cfg.error_handler(|err, _req| {
                        ActixError::InvalidJson(format!("{}", err)).into()
                    });
                    query_cfg.error_handler(|err, _req| {
                        ActixError::InvalidQueryParam(format!("{}", err)).into()
                    });
                },
            );
        })
        .resource("/status", |r| r.with(status))
    // .resource("/features", |r| r.with(features))
    // .resource("/reverse", |r| r.with(reverse))
    .resource("/metrics", |r| r.f(metrics))
}

pub fn runserver() {
    let args = Args::from_args();
    let ctx: Context = (&args).into();
    server::new(move || create_server(ctx.clone()))
        .bind(&args.bind)
        .unwrap()
        .run();
}
