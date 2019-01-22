use crate::routes::{autocomplete, entry_point, status};
use crate::{Args, Context};
use actix_web::middleware;
use actix_web::{server, App};
use structopt::StructOpt;

pub fn create_server(ctx: Context) -> App<Context> {
    App::with_state(ctx)
        .middleware(
            middleware::cors::Cors::build()
                .allowed_methods(vec!["GET"])
                .finish(),
        )
        .middleware(middleware::Logger::default())
        .resource("/", |r| r.f(entry_point))
        .resource("/v1/autocomplete", |r| r.with(autocomplete))
        .resource("/status", |r| r.with(status))
    // .resource("/features", |r| r.with(features))
    // .resource("/reverse", |r| r.with(reverse))
    // .resource("/metrics", |r| r.with(metrics))
}

pub fn runserver() {
    let args = Args::from_args();
    let ctx: Context = (&args).into();
    server::new(move || create_server(ctx.clone()))
        .bind(&args.bind)
        .unwrap()
        .run();
}
