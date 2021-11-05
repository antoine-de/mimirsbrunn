use config::Config;
use futures::stream::StreamExt;
use snafu::{ResultExt, Snafu};

use std::path::PathBuf;
use std::sync::Arc;

use common::document::ContainerDocument;
use mimir::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir::domain::model::configuration::root_doctype_dataset;
use mimir::domain::ports::primary::list_documents::ListDocuments;
use mimir::domain::ports::secondary::storage::{Error as StorageError, Storage};
use mimirsbrunn::bano::Bano;
use places::addr::Addr;
use places::admin::Admin;

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

pub async fn index_addresses(
    client: &ElasticsearchStorage,
    region: &str,
    dataset: &str,
    reindex_if_already_exists: bool,
) -> Result<Status, Error> {
    // Check if the address index already exists
    let container = root_doctype_dataset(Addr::static_doc_type(), dataset);

    let index = client
        .find_container(container)
        .await
        .context(ContainerSearch)?;

    // If the previous step has been skipped, then we don't need to index BANO file.
    if index.is_some() && !reindex_if_already_exists {
        return Ok(Status::Skipped);
    }

    let base_path = env!("CARGO_MANIFEST_DIR");
    let input_dir: PathBuf = [base_path, "..", "..", "tests", "fixtures", "bano", region]
        .iter()
        .collect();
    let input_file = input_dir.join(format!("{}.csv", region));

    // TODO: there might be some factorisation to do with bano2mimir?
    let into_addr = {
        let admins: Vec<Admin> = client
            .list_documents()
            .await
            .expect("could not query for admins")
            .map(|admin| admin.expect("could not parse admin"))
            .collect()
            .await;

        let admins_by_insee = admins
            .iter()
            .cloned()
            .filter(|addr| !addr.insee.is_empty())
            .map(|addr| (addr.insee.clone(), Arc::new(addr)))
            .collect();

        let admins_geofinder = admins.into_iter().collect();
        move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder)
    };

    // Load file
    let config = Config::builder()
        .add_source(Addr::default_es_container_config())
        .set_override("container.dataset", dataset.to_string())
        .expect("failed to set dataset name")
        .build()
        .expect("failed to build configuration");

    mimirsbrunn::addr_reader::import_addresses_from_input_path(
        client, config, input_file, into_addr,
    )
    .await
    .map_err(|err| Error::Indexing {
        details: format!("could not index bano: {}", err.to_string(),),
    })?;

    Ok(Status::Done)
}
