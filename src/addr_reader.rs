use flate2::bufread::GzDecoder;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use tracing::warn;

use crate::utils;
use places::addr::Addr;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("CSV Error: {}", source))]
    Csv { source: csv_async::Error },

    #[snafu(display("IO Error: {}", source))]
    InvalidIO { source: tokio::io::Error },

    #[snafu(display("Path does not exist: {}", source))]
    InvalidPath { source: tokio::io::Error },

    #[snafu(display("Invalid extention"))]
    InvalidExtention,
}

/// Import the addresses found in path, using the given (Elastiscsearch) configuration and client.
/// The function `into_addr` is used to transform the item read in the file (Bano) into an actual
/// address.
pub fn import_addresses_from_input_path<F, T>(
    path: &PathBuf,
    has_headers: bool,
    into_addr: F,
) -> impl Iterator<Item = Addr>
where
    F: Fn(T) -> Result<Addr, crate::error::Error> + 'static,
    T: DeserializeOwned + 'static,
{
    records_from_directory(&path, has_headers)
        .filter_map(move |record| match record {
            Ok(value) => Some(into_addr(value)),
            Err(err) => {
                warn!("invalid CSV record: {err}");
                None
            }
        })
        .filter_map(|res_addr| match res_addr {
            Ok(addr) => Some(addr),
            Err(err) => {
                warn!("Invalid address has been ignored: {err}");
                None
            }
        })
        .filter(|addr| {
            let empty_name = addr.street.name.is_empty();

            if empty_name {
                warn!(
                    "Address {} has no street name and has been ignored.",
                    addr.id
                )
            }

            !empty_name
        })
}

/// Same as records_from_file, but can take an entire directory as input
fn records_from_directory<T>(
    path: &Path,
    has_headers: bool,
) -> impl Iterator<Item = Result<T, Error>> + 'static
where
    T: DeserializeOwned + 'static,
{
    utils::fs::walk_files_recursive(path).flat_map(move |path| {
        records_from_path(&path, has_headers).unwrap_or_else(|err| {
            warn!("skipping invalid file {:?}: {}", path, err);
            Box::new(std::iter::empty())
        })
    })
}

fn records_from_path<T>(
    path: &Path,
    has_headers: bool,
) -> Result<Box<dyn Iterator<Item = Result<T, Error>> + 'static>, Error>
where
    T: DeserializeOwned + 'static,
{
    let file = std::fs::File::open(path).map_err(|_| Error::InvalidExtention)?;
    let file_read = std::io::BufReader::new(file);
    if path.extension().and_then(OsStr::to_str) == Some("csv") {
        let records = csv::ReaderBuilder::new()
            .has_headers(has_headers)
            .from_reader(file_read);
        Ok(Box::new(
            records
                .into_deserialize()
                .map(|record| record.map_err(|_| Error::InvalidExtention)),
        ))
    } else if path.extension().and_then(OsStr::to_str) == Some("gz") {
        let file_read = GzDecoder::new(file_read);
        let records = csv::ReaderBuilder::new()
            .has_headers(has_headers)
            .from_reader(file_read);
        Ok(Box::new(
            records
                .into_deserialize()
                .map(|record| record.map_err(|_| Error::InvalidExtention)),
        ))
    } else {
        Err(Error::InvalidExtention)
    }
}
