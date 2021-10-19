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
        r#"ntfs file has been indexed for (.*) as (.*)"#,
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx.matches[2].clone();

            state
                .execute(IndexNtfs { region, dataset }, &ctx)
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
#[derive(Debug, PartialEq)]
pub struct IndexNtfs {
    pub region: String,
    pub dataset: String,
}

#[async_trait(?Send)]
impl Step for IndexNtfs {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region, dataset } = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&IndexCosmogony {
                region: region.to_string(),
                dataset: dataset.to_string(),
            })
            .expect("You must index admins before indexing stops");

        // FIXME Other requirements?

        // Check if the stop index already exists
        let container = root_doctype_dataset(Stop::static_doc_type(), dataset);

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
            .set_override("container.dataset", dataset.to_string())
            .expect("failed to set dataset name")
            .build()
            .expect("failed to build configuration");

        let base_path = env!("CARGO_MANIFEST_DIR");
        let input_dir: PathBuf = [base_path, "tests", "fixtures", "ntfs", region]
            .iter()
            .collect();

        index_ntfs(input_dir, config, client)
            .await
            .expect("error while indexing Ntfs");

        Ok(StepStatus::Done)
    }
}
