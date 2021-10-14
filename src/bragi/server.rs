use snafu::{ResultExt, Snafu};
use std::net::ToSocketAddrs;
use tracing::{info, instrument};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use warp::Filter;

use super::settings::{Error as SettingsError, Opts, Settings};
use mimir2::{
    adapters::primary::bragi::api::{forward_geocoder, reverse_geocoder, status},
    adapters::primary::bragi::{handlers, routes},
    adapters::secondary::elasticsearch::remote::{
        connection_pool_url, Error as ElasticsearchRemoteError,
    },
    domain::ports::secondary::remote::{Error as PortRemoteError, Remote},
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not create an Elasticsearch Connection Pool: {}", source))]
    ElasticsearchConnectionPoolCreation { source: ElasticsearchRemoteError },

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

    #[snafu(display("Could not init log file: {}", source))]
    InitLog { source: std::io::Error },
}

#[allow(clippy::needless_lifetimes)]
pub async fn run(opts: &Opts) -> Result<(), Error> {
    let settings = Settings::new(opts).context(SettingsProcessing)?;
    LogTracer::init().expect("Unable to setup log tracer!");

    // Filter traces based on the RUST_LOG env var, or, if it's not set,
    // default to show the output of the example.
    let filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "tracing=info,mimir2=debug".to_owned());

    // following code mostly from https://betterprogramming.pub/production-grade-logging-in-rust-applications-2c7fffd108a6
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();

    // tracing_appender::non_blocking()
    let (non_blocking, _guard) = {
        if settings.logging.path.is_dir() {
            let file_appender =
                tracing_appender::rolling::daily(&settings.logging.path, "mimir.log");

            tracing_appender::non_blocking(file_appender)
        } else {
            tracing_appender::non_blocking(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&settings.logging.path)
                    .context(InitLog)?,
            )
        }
    };

    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking);
    let subscriber = Registry::default()
        .with(EnvFilter::new(&filter))
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);
    tracing::subscriber::set_global_default(subscriber).expect("tracing subscriber global default");

    run_server(settings).await
}

#[allow(clippy::needless_lifetimes)]
pub async fn config(opts: &Opts) -> Result<(), Error> {
    let settings = Settings::new(opts).context(SettingsProcessing)?;
    println!("{}", serde_json::to_string_pretty(&settings).unwrap());
    Ok(())
}

#[instrument(skip(settings))]
pub async fn run_server(settings: Settings) -> Result<(), Error> {
    let addr = settings
        .elasticsearch
        .url
        .socket_addrs(|| Some(9200))
        .expect("invalid elasticsearch url") // TODO: catch error correctly
        .pop()
        .ok_or(Error::AddrResolution {
            msg: "Cannot resolve elasticsearch addr.".to_string(),
        })?;

    let elasticsearch_url = format!("http://{}", addr);
    info!("Connecting to Elasticsearch at {}", &elasticsearch_url);

    let pool = connection_pool_url(&elasticsearch_url)
        .await
        .context(ElasticsearchConnectionPoolCreation)?;

    let client = pool
        .conn(settings.elasticsearch)
        .await
        .context(ElasticsearchConnection)?;

    // Here I place reverse_geocoder first because its most likely to get hit.
    let api = reverse_geocoder!(client.clone(), settings.query.clone())
        .or(forward_geocoder!(client.clone(), settings.query))
        .or(status!(client, elasticsearch_url))
        .recover(routes::report_invalid)
        .with(warp::trace::request());

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
