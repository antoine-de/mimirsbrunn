use crate::extractors::ActixError;
// use crate::prometheus_middleware;
use crate::routes::{
    autocomplete, entry_point, features, metrics, post_autocomplete, reverse, status,
};
use crate::{Args, Context};
use actix_service::{IntoNewService, NewService};
use actix_web::{body::Body, dev, error::Error, middleware, web, App, HttpRequest, HttpServer};
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
            .route(web::post().to(post_autocomplete)),
    )
    // TODO
    // .with_config(post_autocomplete, |(_, _, json_cfg)| {
    //     json_cfg.error_handler(|err, _req| {
    //         ActixError::InvalidJson(format!("{}", err)).into()
    //     });
    // });
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
    )
    .service(
        web::resource("/metrics")
            .name("metrics")
            .route(web::get().to(metrics)),
    );
}

// pub fn create_server<T, B>(ctx: Context) -> App<T, B> where
//     B: MessageBody,
//     T: NewService<Config = (), Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = Error, InitError = ()>,   {

// }

pub fn runserver() -> std::io::Result<()> {
    let args = Args::from_args();
    let ctx: Context = (&args).into();
    let prometheus = actix_web_prom::PrometheusMetrics::new("api", "/metrics"); //TODO don't forget to add in_flight queries
    HttpServer::new(move || {
        App::new()
            .data(ctx.clone())
            .wrap(actix_cors::Cors::new().allowed_methods(vec!["GET"]))
            .wrap(prometheus.clone())
            // .wrap(prometheus_middleware::PrometheusMiddleware::default())
            .wrap(middleware::Logger::default())
            .configure(configure_server)
            .default_service(web::resource("").route(web::get().to(default_404)))
    })
    .bind(&args.bind)?
    .workers(args.nb_threads)
    .run()
}
