use flate2::read::GzDecoder;
use futures::future;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use snafu::futures::TryStreamExt as SnafuTryStreamExt;
use snafu::{ResultExt, Snafu};
use std::io::Read;
use std::marker::{Send, Sync};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tracing::{info, warn};

use config::Config;
use mimir2::{
    adapters::secondary::elasticsearch::ElasticsearchStorage,
    domain::{model::index::IndexVisibility, ports::primary::generate_index::GenerateIndex},
};
use places::addr::Addr;

#[derive(Debug, Snafu)]
pub enum Error {
    // FIXME: I cannot use this here, because the model error contains Box<dyn stdError>
    // which makes a stream of Result<T, Error> not `Send`.
    // #[snafu(display("Index Generation Error: {}", source))]
    // IndexGeneration {
    //     source: mimir2::domain::model::error::Error,
    // },
    #[snafu(display("CSV Error: {}", source))]
    Csv { source: csv_async::Error },

    #[snafu(display("IO Error: {}", source))]
    InvalidIO { source: tokio::io::Error },
}

async fn import_addresses<S, F, T>(
    client: &ElasticsearchStorage,
    config: Config,
    records: S,
    into_addr: F,
) -> Result<(), Error>
where
    T: Send + Sync + 'static,
    F: Fn(T) -> Result<Addr, crate::Error> + Send + Sync + 'static,
    S: Stream<Item = T> + Send + Sync + 'static,
{
    let into_addr = Arc::new(into_addr);

    let addrs = records
        .chunks(1000)
        .map(move |addresses| {
            let into_addr = into_addr.clone();

            async move {
                tokio::task::spawn_blocking(move || {
                    let addresses = addresses
                        .into_iter()
                        .map(|addr| into_addr(addr))
                        .filter_map(|addr_res| match addr_res {
                            Err(err) => {
                                warn!("Address Error ignored: {}", err);
                                None
                            }
                            Ok(addr) if addr.street.name.is_empty() => {
                                warn!(
                                    "Address {} has no street name and has been ignored.",
                                    addr.id
                                );
                                None
                            }
                            Ok(addr) => Some(addr),
                        })
                        .collect::<Vec<_>>();

                    futures::stream::iter(addresses)
                })
                .await
                .expect("tokio thread panicked")
            }
        })
        .buffered(num_cpus::get())
        .flatten();

    client
        .generate_index(config, addrs, IndexVisibility::Public)
        .await
        .unwrap();
    // FIXME See above in definition of the enum Error.
    //.context(IndexGeneration)?;

    Ok(())
}

pub async fn import_addresses_from_reads<T, F>(
    client: &ElasticsearchStorage,
    config: Config,
    has_headers: bool,
    _nb_threads: usize,
    inputs: Vec<impl Read + Send + Sync + 'static>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, crate::Error> + Send + Sync + 'static,
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
    client: &ElasticsearchStorage,
    config: Config,
    has_headers: bool,
    nb_threads: usize,
    files: impl IntoIterator<Item = PathBuf>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, crate::Error> + Send + Sync + 'static,
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

/// Import the addresses found in path, using the given (Elastiscsearch) configuration and client.
/// The function `into_addr` is used to transform the item read in the file (Bano) into an actual
/// address.
pub async fn import_addresses_from_input_path<F, T>(
    client: &ElasticsearchStorage,
    config: Config,
    path: PathBuf,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, crate::Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    if path.is_dir() {
        let files = dir_to_stream(path).await?;
        let records = files
            .filter_map(move |file| async move {
                match file {
                    Ok(file) => match stream_records_from_file(file.clone()).await {
                        Ok(records) => Some(records.filter_map(|rec| future::ready(rec.ok()))),
                        Err(err) => {
                            warn!(
                                "could not stream records from {}: {}",
                                file.display(),
                                err.to_string()
                            );
                            None
                        }
                    },
                    Err(err) => {
                        warn!("directory entry error: {}", err.to_string());
                        None
                    }
                }
            })
            .flatten();

        import_addresses(client, config, records, into_addr).await
    } else {
        let records = stream_records_from_file(path).await?;
        let records = records.filter_map(|rec| future::ready(rec.ok()));
        import_addresses(client, config, records, into_addr).await
    }
}

// Turns a directory into a Stream of PathBuf
async fn dir_to_stream(
    dir: PathBuf,
) -> Result<impl Stream<Item = Result<PathBuf, Error>> + Unpin, Error> {
    let entries = tokio::fs::read_dir(dir).await.unwrap();

    let stream = tokio_stream::wrappers::ReadDirStream::new(entries);

    Ok(stream.map_ok(|entry| entry.path()).context(InvalidIO))
}

async fn stream_records_from_file<T>(
    file: PathBuf,
) -> Result<impl Stream<Item = Result<T, Error>> + Send + Sync + Unpin + 'static, Error>
where
    T: DeserializeOwned + Send + Sync + 'static,
{
    let reader = File::open(file).await.context(InvalidIO)?;

    let csv_reader = csv_async::AsyncReaderBuilder::new()
        .has_headers(false)
        .create_deserializer(reader);
    Ok(csv_reader.into_deserialize::<T>().context(Csv))
}
