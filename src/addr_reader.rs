use crate::Error;
use csv;
use failure::ResultExt;
use flate2::read::GzDecoder;
use mimir::rubber::{IndexSettings, IndexVisibility, Rubber};
use mimir::Addr;
use par_map::ParMap;
use serde::de::DeserializeOwned;
use slog_scope::{error, info, warn};
use std::fs::File;
use std::io::Read;
use std::marker::{Send, Sync};
use std::path::PathBuf;

fn import_addresses<T, F>(
    rubber: &mut Rubber,
    nb_threads: usize,
    index_settings: IndexSettings,
    dataset: &str,
    addresses: impl IntoIterator<Item = T>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let addr_index = rubber
        .make_index(dataset, &index_settings)
        .with_context(|_| format!("Error occurred when making index {}", dataset))?;

    info!("Add data in elasticsearch db.");
    let iter = addresses
        .into_iter()
        .with_nb_threads(nb_threads)
        .par_map(into_addr)
        .filter_map(|ra| match ra {
            Ok(a) => {
                if a.street.name.is_empty() {
                    warn!("Address {} has no street name and has been ignored.", a.id);
                    None
                } else {
                    Some(a)
                }
            }
            Err(err) => {
                warn!("Address Error ignored: {}", err);
                None
            }
        });

    let nb = rubber
        .bulk_index(&addr_index, iter)
        .with_context(|err| format!("failed to bulk insert: {}", err))?;
    info!("importing addresses: {} addresses added.", nb);

    rubber
        .publish_index(dataset, addr_index, IndexVisibility::Public)
        .context("Error while publishing the index")?;
    Ok(())
}

pub fn import_addresses_from_streams<T, F>(
    rubber: &mut Rubber,
    has_headers: bool,
    nb_threads: usize,
    index_settings: IndexSettings,
    dataset: &str,
    streams: impl IntoIterator<Item = impl Read>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let iter = streams
        .into_iter()
        .flat_map(|stream| {
            csv::ReaderBuilder::new()
                .has_headers(has_headers)
                .from_reader(stream)
                .into_deserialize()
        })
        .filter_map(|line| {
            line.map_err(|e| warn!("Impossible to read line, error: {}", e))
                .ok()
        });

    import_addresses(rubber, nb_threads, index_settings, dataset, iter, into_addr)
}

pub fn import_addresses_from_files<T, F>(
    rubber: &mut Rubber,
    has_headers: bool,
    nb_threads: usize,
    index_settings: IndexSettings,
    dataset: &str,
    files: impl IntoIterator<Item = PathBuf>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let streams = files.into_iter().filter_map(|path| {
        info!("importing {:?}...", &path);

        let with_gzip = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "gz")
            .unwrap_or(false);

        File::open(&path)
            .map_err(|err| error!("Impossible to read file {:?}, error: {}", path, err))
            .map(|file| {
                if with_gzip {
                    let decoder = GzDecoder::new(file);
                    Box::new(decoder) as Box<dyn Read>
                } else {
                    Box::new(file) as Box<dyn Read>
                }
            })
            .ok()
    });

    import_addresses_from_streams(
        rubber,
        has_headers,
        nb_threads,
        index_settings,
        dataset,
        streams,
        into_addr,
    )
}
