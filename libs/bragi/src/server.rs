use crate::extractors::ActixError;
use crate::prometheus_middleware;
use crate::routes::{
    autocomplete, entry_point, features, metrics, post_autocomplete, reverse, status,
};
use crate::{Args, Context};
use actix_web::{http, middleware, server, App, HttpRequest, Json};
use failure::Error;
use structopt::StructOpt;

fn default_404(req: &HttpRequest<Context>) -> Result<Json<()>, ActixError> {
    Err(ActixError::RouteNotFound(req.path().to_string()))
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
        .resource("/", |r| {
            r.name("/");
            r.f(entry_point)
        })
        .resource("/autocomplete", |r| {
            r.name("autocomplete");
            r.method(http::Method::GET).with(autocomplete);
            r.method(http::Method::POST)
                .with_config(post_autocomplete, |(_, _, json_cfg)| {
                    json_cfg.error_handler(|err, _req| {
                        ActixError::InvalidJson(format!("{}", err)).into()
                    });
                });
        })
        .resource("/status", |r| {
            r.name("status");
            r.with(status)
        })
        .resource("/features/{id}", |r| {
            r.name("features");
            r.with(features)
        })
        .resource("/reverse", |r| {
            r.name("reverse");
            r.with(reverse)
        })
        .resource("/metrics", |r| {
            r.name("metrics");
            r.f(metrics)
        })
        .default_resource(|r| {
            // custom error for 404
            r.f(default_404)
        })
}

pub fn runserver() -> Result<(), Error> {
    let args = Args::from_args();
    // let ctx: Context = (&args).into();
    let bob = args.clone();
    Ok(server::new(move || create_server((&bob.clone()).into()))
        .bind(&args.bind)?
        .workers(args.nb_threads)
        .run())
}
