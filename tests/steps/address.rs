use crate::error::Error;
use crate::steps::admin::IndexCosmogony;
use crate::{error, State, Step, StepStatus};
use async_trait::async_trait;
use common::document::ContainerDocument;
use config::Config;
use cucumber::{t, Steps};
use futures::stream::StreamExt;
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use mimir2::adapters::secondary::elasticsearch::{ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ};
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use mimirsbrunn::addr_reader::import_addresses_from_file;
use mimirsbrunn::bano::Bano;
use places::addr::Addr;
use places::admin::Admin;
use snafu::ResultExt;
use std::path::PathBuf;
use std::sync::Arc;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "bano file has been indexed for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute(IndexBano(region))
                .await
                .expect("failed to index Bano file");

            state
        }),
    );

    steps
}

/// Index a bano file for given region into ES.
///
/// This will require to import admins first.
#[derive(PartialEq)]
pub struct IndexBano(String);

#[async_trait(?Send)]
impl Step for IndexBano {
    async fn execute(&mut self, state: &State) -> Result<StepStatus, Error> {
        let Self(region) = self;

        state
            .status_of(&IndexCosmogony(region.to_string()))
            .expect("You must index admins before indexing addresses");

        // Connect to Elasticsearch
        let pool = connection_test_pool()
            .await
            .context(error::ElasticsearchPool {
                details: "Could not retrieve Elasticsearch test pool".to_string(),
            })?;

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .context(error::ElasticsearchConnection {
                details: "Could not establish connection to Elasticsearch".to_string(),
            })?;

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
            move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder, false)
        };

        // Load file
        let config = Config::builder()
            .add_source(Addr::default_es_container_config())
            .set_override("name", "test_addr")
            .expect("failed to set index name in config")
            .build()
            .expect("failed to build configuration");

        let base_path = env!("CARGO_MANIFEST_DIR");
        let input_dir: PathBuf = [base_path, "tests", "data", "bano"].iter().collect();
        let input_file = input_dir.join(format!("{}.csv", region));

        import_addresses_from_file(client, config, input_file, into_addr)
            .await
            .expect("error while indexing Bano");

        Ok(StepStatus::Done)
    }
}
