use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

use super::utils::{create_dir_if_not_exists, file_exists};
use common::document::ContainerDocument;
use mimir::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir::domain::model::configuration::root_doctype_dataset;
use mimir::domain::ports::secondary::storage::{Error as StorageError, Storage};
use places::admin::Admin;

use super::utils;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Could not Create Directory: {}", source))]
    CreateDir { source: utils::Error },
    #[snafu(display("Invalid IO: {} ({})", source, details))]
    InvalidIO {
        details: String,
        source: std::io::Error,
    },
    #[snafu(display("Invalid JSON: {} ({})", source, details))]
    Json {
        details: String,
        source: serde_json::Error,
    },
    #[snafu(display("NTFS Dataset not found"))]
    NtfsDatasetNotFound,
    #[snafu(display("Environment Variable Error: {} ({})", source, details))]
    EnvironmentVariable {
        details: String,
        source: std::env::VarError,
    },
    #[snafu(display("Indexing Error: {}", details))]
    Indexing { details: String },

    #[snafu(display("Container Search Error: {}", source))]
    ContainerSearch { source: StorageError },
}

pub enum Status {
    Skipped,
    Done,
}

pub async fn generate(region: &str, regenerate_if_already_exists: bool) -> Result<Status, Error> {
    // Build
    let base_path = env!("CARGO_MANIFEST_DIR");

    let input_dir: PathBuf = [base_path, "..", "..", "tests", "fixtures", "osm", region]
        .iter()
        .collect();
    let input_file = input_dir.join(format!("{}-latest.osm.pbf", region));

    let output_dir: PathBuf = [
        base_path,
        "..",
        "..",
        "tests",
        "fixtures",
        "cosmogony",
        region,
    ]
    .iter()
    .collect();
    let output_file = output_dir.join(format!("{}.jsonl.gz", region));

    create_dir_if_not_exists(&output_dir)
        .await
        .context(CreateDir)?;

    // If the output already exists, and we don't need to regenerate it, skip this step.
    if file_exists(&output_file).await && !regenerate_if_already_exists {
        return Ok(Status::Skipped);
    }

    // FIXME It would be nice not to resort to an env variable, and to use cosmogony as a library.
    let cosmogony_path = std::env::var("COSMOGONY_EXE").context(EnvironmentVariable {
        details: "Could not get COSMOGONY_EXE environment variable".to_string(),
    })?;

    // TODO: check command status ?
    tokio::process::Command::new(&cosmogony_path)
        .args(["--country-code", "FR"])
        .arg("--input")
        .arg(&input_file)
        .arg("--output")
        .arg(&output_file)
        .spawn()
        .expect("failed to spawn cosmogony")
        .wait()
        .await
        .context(InvalidIO {
            details: format!(
                "failed to generate cosmogony with input {} and output {}",
                input_file.display(),
                output_file.display()
            ),
        })?;

    Ok(Status::Done)
}

pub async fn index_admins(
    client: &ElasticsearchStorage,
    region: &str,
    dataset: &str,
    reindex_if_already_exists: bool,
) -> Result<Status, Error> {
    // Check if the admin index already exists
    let container = root_doctype_dataset(Admin::static_doc_type(), dataset);

    let index = client
        .find_container(container.clone())
        .await
        .context(ContainerSearch)?;

    // If the previous step has been skipped, then we don't need to index the
    // cosmogony file.
    if index.is_some() && !reindex_if_already_exists {
        return Ok(Status::Skipped);
    }

    let base_path = env!("CARGO_MANIFEST_DIR");
    let config_dir: PathBuf = [base_path, "..", "..", "config"].iter().collect();
    let input_dir: PathBuf = [
        base_path,
        "..",
        "..",
        "tests",
        "fixtures",
        "cosmogony",
        region,
    ]
    .iter()
    .collect();
    let input_file = input_dir.join(format!("{}.jsonl.gz", region));

    let config: mimirsbrunn::settings::cosmogony2mimir::Settings = common::config::config_from(
        &config_dir,
        &["cosmogony2mimir", "elasticsearch", "logging"],
        "testing",
        None,
        vec![],
    )
    .expect("could not load cosmogony2mimir configuration")
    .try_into()
    .expect("invalid cosmogony2mimir configuration");

    mimirsbrunn::admin::index_cosmogony(
        &input_file,
        vec!["fr".to_string()],
        &config.container,
        client,
    )
    .await
    .map_err(|err| Error::Indexing {
        details: format!("could not index cosmogony: {}", err.to_string(),),
    })?;

    Ok(Status::Done)
}
