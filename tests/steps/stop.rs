use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use crate::steps::admin::IndexCosmogony;
use async_trait::async_trait;
use common::document::ContainerDocument;
use config::Config;
use cucumber::{t, StepContext, Steps};
use mimir2::{
    adapters::secondary::elasticsearch::ElasticsearchStorage,
    domain::{model::configuration::root_doctype_dataset, ports::secondary::storage::Storage},
};
use mimirsbrunn::stops::index_ntfs;
use places::stop::Stop;
use std::path::PathBuf;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "ntfs file has been indexed for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute(IndexNtfs(region), &ctx)
                .await
                .expect("failed to index Ntfs file");

            state
        }),
    );

    steps
}

/// Index an ntfs file for a given region into Elasticsearch.
///
/// This will require to import admins first.
#[derive(PartialEq)]
pub struct IndexNtfs(String);

#[async_trait(?Send)]
impl Step for IndexNtfs {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self(region) = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&IndexCosmogony(region.to_string()))
            .expect("You must index admins before indexing stops");

        // FIXME Other requirements?

        // Check if the stop index already exists
        let container = root_doctype_dataset(Stop::static_doc_type(), region);

        let index = client
            .find_container(container)
            .await
            .expect("failed at looking up for container");

        // If we find an existing index, then we skip this indexation step.
        if index.is_some() {
            return Ok(StepStatus::Skipped);
        }

        // Load file
        let config = Config::builder()
            .add_source(Stop::default_es_container_config())
            .set_override("container.dataset", region.to_string())
            .expect("failed to set dataset name")
            .build()
            .expect("failed to build configuration");

        let base_path = env!("CARGO_MANIFEST_DIR");
        let input_dir: PathBuf = [base_path, "tests", "fixtures", "ntfs"].iter().collect();

        index_ntfs(input_dir, config, client)
            .await
            .expect("error while indexing Ntfs");

        Ok(StepStatus::Done)
    }
}
