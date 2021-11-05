use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use snafu::ResultExt;

use crate::error::{self, Error};
use crate::state::{State, Step, StepStatus};
use crate::steps::admin::IndexCosmogony;
use crate::steps::download::DownloadOsm;
use mimir::adapters::secondary::elasticsearch::ElasticsearchStorage;
use tests::osm;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "streets have been indexed for (.*) as (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx.matches[2].clone();

            state
                .execute(DownloadOsm(region.clone()), &ctx)
                .await
                .expect("failed to download OSM file");

            state
                .execute(IndexStreets { region, dataset }, &ctx)
                .await
                .expect("failed to index OSM file for streets");

            state
        }),
    );

    steps
}

/// Index an osm file for a given region into Elasticsearch, extracting streets
///
/// This will require to import admins first.
#[derive(PartialEq)]
pub struct IndexStreets {
    pub region: String,
    pub dataset: String,
}

#[async_trait(?Send)]
impl Step for IndexStreets {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region, dataset } = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&IndexCosmogony {
                region: region.to_string(),
                dataset: dataset.to_string(),
            })
            .expect("You must index admins before indexing stops");

        osm::index_streets(client, region, dataset, false)
            .await
            .map(|status| status.into())
            .context(error::IndexOsm)
    }
}
