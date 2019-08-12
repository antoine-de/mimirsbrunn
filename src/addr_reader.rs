use crate::Error;
use csv;
use failure::ResultExt;
use mimir::rubber::{IndexSettings, IndexVisibility, Rubber};
use mimir::Addr;
use par_map::ParMap;
use serde::de::DeserializeOwned;
use slog::{slog_debug, slog_error, slog_info};
use slog_scope::{debug, error, info};
use std::marker::{Send, Sync};
use std::path::PathBuf;

pub fn import_addresses<T, F>(
    rubber: &mut Rubber,
    has_headers: bool,
    nb_threads: usize,
    index_settings: IndexSettings,
    dataset: &str,
    files: impl IntoIterator<Item = PathBuf>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Addr + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let addr_index = rubber
        .make_index(dataset, &index_settings)
        .with_context(|_| format!("Error occurred when making index {}", dataset))?;
    info!("Add data in elasticsearch db.");

    let iter = files
        .into_iter()
        .flat_map(|f| {
            info!("importing {:?}...", &f);
            csv::ReaderBuilder::new()
                .has_headers(has_headers)
                .from_path(&f)
                .map_err(|e| error!("impossible to read file {:?}, error: {}", f, e))
                .ok()
                .into_iter()
                .flat_map(|rdr| {
                    rdr.into_deserialize().filter_map(|r| {
                        r.map_err(|e| info!("impossible to read line, error: {}", e))
                            .ok()
                    })
                })
        })
        .with_nb_threads(nb_threads)
        .par_map(into_addr)
        .filter(|a| {
            !a.street.name.is_empty() || {
                debug!("Address {} has no street name and has been ignored.", a.id);
                false
            }
        });
    let nb = rubber
        .bulk_index(&addr_index, iter)
        .with_context(|_| format!("failed to bulk insert"))?;
    info!("importing addresses: {} addresses added.", nb);

    rubber
        .publish_index(dataset, addr_index, IndexVisibility::Public)
        .context("Error while publishing the index")?;
    Ok(())
}
