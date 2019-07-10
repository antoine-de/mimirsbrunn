use crate::extractors::ActixError;
use crate::routes::{
    autocomplete, entry_point, features, post_autocomplete, reverse, status, JsonParams,
};
use crate::{Args, Context};
use actix_web::FromRequest;
use actix_web::{middleware, web, App, HttpRequest, HttpServer};
use structopt::StructOpt;

pub fn default_404(req: HttpRequest) -> Result<web::Json<()>, ActixError> {
    Err(ActixError::RouteNotFound(req.path().to_string()))
}

pub fn configure_server(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/")
            .name("/")
            .route(web::get().to(entry_point)),
    )
    .service(
        web::resource("/autocomplete")
            .name("autocomplete")
            .route(web::get().to(autocomplete))
            .route(web::post().to(post_autocomplete))
            .data(web::Json::<JsonParams>::configure(|cfg| {
                cfg.error_handler(|err, _req| ActixError::InvalidJson(format!("{}", err)).into())
            })),
    )
    .service(
        web::resource("/status")
            .name("status")
            .route(web::get().to(status)),
    )
    .service(
        web::resource("/features/{id}")
            .name("features")
            .route(web::get().to(features)),
    )
    .service(
        web::resource("/reverse")
            .name("reverse")
            .route(web::get().to(reverse)),
    );
}

pub fn runserver() -> std::io::Result<()> {
    let args = Args::from_args();
    let ctx: Context = (&args).into();
    let prometheus = crate::prometheus_middleware::PrometheusMetrics::new("bragi", "/metrics");
    HttpServer::new(move || {
        App::new()
            .data(ctx.clone())
            // NOTE: if some middlewares are added, don't forget to add them in the tests too (in BragiHandler::new)
            .wrap(actix_cors::Cors::new().allowed_methods(vec!["GET"]))
            .wrap(prometheus.clone())
            .wrap(middleware::Logger::default())
            .configure(configure_server)
            .default_service(web::resource("").route(web::get().to(default_404)))
    })
    .bind(&args.bind)?
    .workers(args.nb_threads)
    .run()
}
