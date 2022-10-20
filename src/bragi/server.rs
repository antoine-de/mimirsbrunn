use mimirsbrunn::utils::logger::logger_init;
use snafu::{ResultExt, Snafu};
use std::net::ToSocketAddrs;
use tokio::runtime;
use tracing::{info, instrument};
use warp::{path, Filter};

use super::settings::{Error as SettingsError, Opts};
use mimir::adapters::primary::bragi::api::{ForwardGeocoderExplainQuery, ReverseGeocoderQuery};
use mimir::adapters::primary::bragi::handlers::{self, Settings};
use mimir::adapters::primary::bragi::prometheus_handler::update_metrics;
use mimir::adapters::primary::bragi::routes::{self, validate_forward_geocoder};
use mimir::adapters::secondary::elasticsearch::remote::connection_pool_url;
use mimir::domain::ports::secondary::remote::{Error as PortRemoteError, Remote};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not establish Elasticsearch Connection: {}", source))]
    ElasticsearchConnection { source: PortRemoteError },

    #[snafu(display("Could not generate settings: {}", source))]
    SettingsProcessing { source: SettingsError },

    #[snafu(display("Socket Addr Error with host {} / port {}: {}", host, port, source))]
    SockAddr {
        host: String,
        port: u16,
        source: std::io::Error,
    },

    #[snafu(display("Addr Resolution Error {}", msg))]
    AddrResolution { msg: String },

    #[snafu(display("Could not init logger: {}", source))]
    InitLog {
        source: mimirsbrunn::utils::logger::Error,
    },
}

pub fn run(opts: &Opts) -> Result<(), Error> {
    let settings: Settings = opts.try_into().context(SettingsProcessingSnafu)?;
    let _log_guard = logger_init().map_err(|err| Error::InitLog { source: err })?;

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(settings.nb_threads.unwrap_or_else(num_cpus::get))
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime.");

    runtime.block_on(run_server(settings))
}

pub fn config(opts: &Opts) -> Result<(), Error> {
    let settings: Settings = opts.try_into().context(SettingsProcessingSnafu)?;
    println!("{}", serde_json::to_string_pretty(&settings).unwrap());
    Ok(())
}

#[instrument(skip(settings))]
pub async fn run_server(settings: Settings) -> Result<(), Error> {
    info!(
        "Connecting to Elasticsearch at {}",
        &settings.elasticsearch.url
    );

    // Wrap Elasticsearch client and settings in a context that will be accessible for all
    // handlers.
    let ctx_builder = {
        let client = connection_pool_url(&settings.elasticsearch.url)
            .conn(settings.elasticsearch.clone())
            .await
            .context(ElasticsearchConnectionSnafu)?;

        let settings = settings.clone();
        let ctx = handlers::Context { client, settings };

        move || {
            let ctx = ctx.clone();
            move || ctx.clone()
        }
    };

    let endpoints = {
        path!("api" / "v1" / "autocomplete")
            .map(ctx_builder())
            .and(validate_forward_geocoder())
            .and_then(handlers::forward_geocoder)
    }
    .or({
        warp::get()
            .and(path!("api" / "v1" / "reverse"))
            .map(ctx_builder())
            .and(ReverseGeocoderQuery::validate())
            .and_then(handlers::reverse_geocoder)
    })
    .or({
        warp::get()
            .and(path!("api" / "v1" / "autocomplete-explain"))
            .map(ctx_builder())
            .and(ForwardGeocoderExplainQuery::validate())
            .and(warp::any().map(|| None)) // the shape is None
            .and_then(handlers::forward_geocoder_explain)
    })
    .or({
        warp::get()
            .and(path!("api" / "v1" / "status"))
            .map(ctx_builder())
            .and_then(handlers::status)
    })
    .or({
        warp::get()
            .and(path!("api" / "v1" / "metrics"))
            .and_then(handlers::metrics)
    });

    let api = endpoints
        .recover(routes::report_invalid)
        .with(warp::wrap_fn(|filter| {
            routes::cache_filter(filter, settings.http_cache_duration)
        }))
        .with(warp::log::custom(update_metrics))
        .with(warp::trace(|info| {
            tracing::info_span!(
                "request",
                method = %info.method(),
                path = %info.path(),
            )
        }));

    info!("api ready");

    let host = settings.service.host;
    let port = settings.service.port;
    let addr = (host.as_str(), port);
    let addr = addr
        .to_socket_addrs()
        .context(SockAddrSnafu { host, port })?
        .next()
        .ok_or(Error::AddrResolution {
            msg: String::from("Cannot resolve bragi addr."),
        })?;

    info!("Serving bragi on {}", addr);
    warp::serve(api).run(addr).await;
    Ok(())
}
