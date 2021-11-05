use snafu::{ResultExt, Snafu};
use std::env;
use std::path::Path;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not init log file: {}", source))]
    InitLog { source: std::io::Error },
}

// FIXME Remove all expects
pub fn logger_init<P: AsRef<Path>>(
    path: P,
) -> Result<tracing_appender::non_blocking::WorkerGuard, Error> {
    LogTracer::init().expect("Unable to setup log tracer!");
    let path = path.as_ref();
    // Filter traces based on the RUST_LOG env var, or, if it's not set,
    // default to show the output of the example.
    let filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "tracing=info,mimir=debug".to_owned());

    // following code mostly from https://betterprogramming.pub/production-grade-logging-in-rust-applications-2c7fffd108a6
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();

    // tracing_appender::non_blocking()
    let (non_blocking, guard) = {
        if path.is_dir() {
            let file_appender = tracing_appender::rolling::daily(&path, "mimir.log");

            tracing_appender::non_blocking(file_appender)
        } else {
            tracing_appender::non_blocking(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
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
    Ok(guard)
}
