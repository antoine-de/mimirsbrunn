use crate::Error;
use config::Config;
use failure::format_err;
use flate2::read::GzDecoder;
use futures::future;
use futures::stream::{self, Stream, StreamExt};
use mimir2::{
    adapters::secondary::elasticsearch::ElasticsearchStorage,
    domain::{model::index::IndexVisibility, ports::primary::generate_index::GenerateIndex},
};
use places::addr::Addr;
use serde::de::DeserializeOwned;
use slog_scope::{info, warn};
use std::io::Read;
use std::marker::{Send, Sync};
use std::path::PathBuf;
use tokio::fs::File;

async fn import_addresses<S, F, T>(
    client: ElasticsearchStorage,
    config: Config,
    records: S,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    S: Stream<Item = T> + Send + Sync + Unpin + 'static,
{
    let addrs = records.map(into_addr).filter_map(|ra| match ra {
        Ok(a) => {
            if a.street.name.is_empty() {
                warn!("Address {} has no street name and has been ignored.", a.id);
                future::ready(None)
            } else {
                future::ready(Some(a))
            }
        }
        Err(err) => {
            warn!("Address Error ignored: {}", err);
            future::ready(None)
        }
    });

    client
        .generate_index(config, addrs, IndexVisibility::Public)
        .await
        .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;

    Ok(())
}

pub async fn import_addresses_from_reads<T, F>(
    client: ElasticsearchStorage,
    config: Config,
    has_headers: bool,
    _nb_threads: usize,
    inputs: Vec<impl Read + Send + Sync + 'static>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + Unpin + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    let iter = inputs
        .into_iter()
        .flat_map(move |stream| {
            csv::ReaderBuilder::new()
                .has_headers(has_headers)
                .from_reader(stream)
                .into_deserialize()
        })
        .filter_map(|line| {
            line.map_err(|e| warn!("Impossible to read line, error: {}", e))
                .ok()
        });

    let stream = stream::iter(iter);

    import_addresses(client, config, stream, into_addr).await
}

pub async fn import_addresses_from_files<T, F>(
    client: ElasticsearchStorage,
    config: Config,
    has_headers: bool,
    nb_threads: usize,
    files: impl IntoIterator<Item = PathBuf>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + Unpin + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    let files = files
        .into_iter()
        .filter_map(|path| {
            info!("importing {:?}...", &path);

            let with_gzip = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "gz")
                .unwrap_or(false);

            std::fs::File::open(&path)
                .map(|file| {
                    if with_gzip {
                        let decoder = GzDecoder::new(file);
                        Box::new(decoder) as Box<dyn Read + Send + Sync>
                    } else {
                        Box::new(file) as Box<dyn Read + Send + Sync>
                    }
                })
                .ok()
        })
        .collect();

    import_addresses_from_reads(client, config, has_headers, nb_threads, files, into_addr).await
}

pub async fn import_addresses_from_file<F, T>(
    client: ElasticsearchStorage,
    config: Config,
    file: PathBuf,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    let reader = File::open(file).await.expect("file open");
    let csv_reader = csv_async::AsyncReaderBuilder::new()
        .has_headers(false)
        .create_deserializer(reader);
    let records = csv_reader
        .into_deserialize::<T>()
        .filter_map(|rec| future::ready(rec.ok()));

    import_addresses(client, config, records, into_addr).await
}
