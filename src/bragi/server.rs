use mimirsbrunn::utils::logger::logger_init;
use snafu::{ResultExt, Snafu};
use std::net::ToSocketAddrs;
use tokio::runtime;
use tracing::{info, instrument};
use warp::Filter;

use super::settings::{Error as SettingsError, Opts, Settings};
use mimir::adapters::primary::bragi::prometheus_handler::update_metrics;
use mimir::{
    adapters::primary::bragi::api::{
        features, forward_geocoder, forward_geocoder_explain, reverse_geocoder, status,
    },
    adapters::primary::bragi::{handlers, routes},
    adapters::secondary::elasticsearch::remote::connection_pool_url,
    domain::ports::secondary::remote::{Error as PortRemoteError, Remote},
    metrics,
};

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
    let settings = Settings::new(opts).context(SettingsProcessing)?;

    let _log_guard = logger_init().map_err(|err| Error::InitLog { source: err })?;

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(settings.nb_threads.unwrap_or_else(num_cpus::get))
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime.");

    runtime.block_on(run_server(settings))
}

pub fn config(opts: &Opts) -> Result<(), Error> {
    let settings = Settings::new(opts).context(SettingsProcessing)?;
    println!("{}", serde_json::to_string_pretty(&settings).unwrap());
    Ok(())
}

#[instrument(skip(settings))]
pub async fn run_server(settings: Settings) -> Result<(), Error> {
    info!(
        "Connecting to Elasticsearch at {}",
        &settings.elasticsearch.url
    );

    let client = connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch.clone())
        .await
        .context(ElasticsearchConnection)?;

    // Here I place reverse_geocoder first because its most likely to get hit.
    let api = reverse_geocoder!(
        client.clone(),
        settings.query.clone(),
        settings.reverse_timeout
    )
    .or(forward_geocoder!(
        client.clone(),
        settings.query.clone(),
        settings.autocomplete_timeout
    ))
    .or(features!(client.clone(), settings.features_timeout))
    .or(forward_geocoder_explain!(
        client.clone(),
        settings.query,
        settings.autocomplete_timeout
    ))
    .or(status!(client.clone(), &settings.elasticsearch.url))
    .or(metrics!())
    .recover(routes::report_invalid)
    .with(warp::wrap_fn(|filter| {
        routes::cache_filter(filter, settings.http_cache_duration)
    }))
    .with(warp::log::custom(update_metrics))
    .with(warp::trace(|info| {
        // Create a span using tracing macros
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
        .context(SockAddr { host, port })?
        .next()
        .ok_or(Error::AddrResolution {
            msg: String::from("Cannot resolve bragi addr."),
        })?;

    info!("Serving bragi on {}", addr);

    warp::serve(api).run(addr).await;

    Ok(())
}
