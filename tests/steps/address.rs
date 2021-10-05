use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use crate::steps::admin::IndexCosmogony;
use async_trait::async_trait;
use common::document::ContainerDocument;
use config::Config;
use cucumber::{t, StepContext, Steps};
use futures::stream::StreamExt;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir2::domain::model::configuration::root_doctype_dataset;
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::storage::Storage;
use mimirsbrunn::addr_reader::import_addresses_from_input_path;
use mimirsbrunn::bano::Bano;
use places::addr::Addr;
use places::admin::Admin;
use std::path::PathBuf;
use std::sync::Arc;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "bano file has been indexed for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute(IndexBano(region), &ctx)
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
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self(region) = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&IndexCosmogony(region.to_string()))
            .expect("You must index admins before indexing addresses");

        // Check if the address index already exists
        let container = root_doctype_dataset(Addr::static_doc_type(), region);

        let index = client
            .find_container(container)
            .await
            .expect("failed at looking up for container");

        // If the previous step has been skipped, then we don't need to index BANO file.
        if index.is_some() {
            return Ok(StepStatus::Skipped);
        }

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
            .set_override("container.dataset", region.to_string())
            .expect("failed to set dataset name")
            .build()
            .expect("failed to build configuration");

        let base_path = env!("CARGO_MANIFEST_DIR");
        let input_dir: PathBuf = [base_path, "tests", "data", "bano"].iter().collect();
        let input_file = input_dir.join(format!("{}.csv", region));

        import_addresses_from_input_path(client.clone(), config, input_file, into_addr)
            .await
            .expect("error while indexing Bano");

        Ok(StepStatus::Done)
    }
}
