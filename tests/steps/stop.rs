use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use snafu::ResultExt;

use crate::error::{self, Error};
use crate::state::{State, Step, StepStatus};
use crate::steps::admin::IndexCosmogony;
use crate::steps::download::DownloadNTFS;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use tests::ntfs;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        r#"ntfs file has been indexed for (.*) as (.*)"#,
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx.matches[2].clone();

            state
                .execute(IndexNTFS { region, dataset }, &ctx)
                .await
                .expect("failed to index NTFS file");

            state
        }),
    );

    // download and index ntfs
    steps.given_regex_async(
        "stops have been indexed for (.*) as (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx.matches[2].clone();

            state
                .execute(
                    DownloadNTFS {
                        region: region.clone(),
                    },
                    &ctx,
                )
                .await
                .expect("failed to download NTFS file");

            state
                .execute(IndexNTFS { region, dataset }, &ctx)
                .await
                .expect("failed to index NTFS file");

            state
        }),
    );

    steps
}

/// Index an NTFS file for a given region into Elasticsearch.
///
/// This will require to import admins first.
#[derive(Debug, PartialEq)]
pub struct IndexNTFS {
    pub region: String,
    pub dataset: String,
}

#[async_trait(?Send)]
impl Step for IndexNTFS {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region, dataset } = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&IndexCosmogony {
                region: region.to_string(),
                dataset: dataset.to_string(),
            })
            .expect("You must index admins before indexing stops");

        ntfs::index_stops(client, region, dataset, false)
            .await
            .map(|status| status.into())
            .context(error::IndexNTFS)
    }
}
