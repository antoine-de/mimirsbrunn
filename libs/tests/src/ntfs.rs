use config::Config;
use snafu::{ResultExt, Snafu};

use std::path::PathBuf;

use common::document::ContainerDocument;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir2::domain::model::configuration::root_doctype_dataset;
use mimir2::domain::ports::secondary::storage::{Error as StorageError, Storage};
use places::stop::Stop;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Indexing Error: {}", details))]
    Indexing { details: String },

    #[snafu(display("Container Search Error: {}", source))]
    ContainerSearch { source: StorageError },
}

pub enum Status {
    Skipped,
    Done,
}

pub async fn index_stops(
    client: &ElasticsearchStorage,
    region: &str,
    dataset: &str,
    reindex_if_already_exists: bool,
) -> Result<Status, Error> {
    let container = root_doctype_dataset(Stop::static_doc_type(), dataset);

    let index = client
        .find_container(container)
        .await
        .context(ContainerSearch)?;

    // If the previous step has been skipped, then we don't need to index BANO file.
    if index.is_some() && !reindex_if_already_exists {
        return Ok(Status::Skipped);
    }

    // Load file
    let config = Config::builder()
        .add_source(Stop::default_es_container_config())
        .set_override("container.dataset", dataset.to_string())
        .expect("failed to set dataset name")
        .build()
        .expect("failed to build configuration");

    let base_path = env!("CARGO_MANIFEST_DIR");
    let input_dir: PathBuf = [base_path, "..", "..", "tests", "fixtures", "ntfs", region]
        .iter()
        .collect();

    mimirsbrunn::stops::index_ntfs(input_dir, config, client)
        .await
        .expect("error while indexing Ntfs");

    Ok(Status::Done)
}
