use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use snafu::ResultExt;

use crate::error::{self, Error};
use crate::state::{State, Step, StepStatus};
use crate::steps::admin::IndexCosmogony;
use crate::steps::download::DownloadBano;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use tests::bano;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        r#"bano file has been indexed for ([^\s]*)(?: as (.*))?"#,
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx
                .matches
                .get(2)
                .map(|d| {
                    if d.is_empty() {
                        region.to_string()
                    } else {
                        d.to_string()
                    }
                })
                .unwrap_or_else(|| region.clone())
                .clone();
            assert!(!region.is_empty());
            assert!(!dataset.is_empty());
            state
                .execute(IndexBano { region, dataset }, &ctx)
                .await
                .expect("failed to index Bano file");

            state
        }),
    );

    // This step is a condensed format for download + index bano
    steps.given_regex_async(
        r"addresses \(bano\) have been indexed for (.*) into (.*) as (.*)",
        t!(|mut state, ctx| {
            let departments = ctx.matches[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let region = ctx.matches[2].clone();
            let dataset = ctx.matches[3].clone();

            state
                .execute_once(
                    DownloadBano {
                        departments,
                        region: region.clone(),
                    },
                    &ctx,
                )
                .await
                .expect("failed to download OSM file");
            state
                .execute(IndexBano { region, dataset }, &ctx)
                .await
                .expect("failed to index Bano file");

            state
        }),
    );

    // This step is a condensed format for download + index bano
    steps.given_regex_async(
        r"addresses \(bano\) have been indexed for (.*) into (.*) as (.*)",
        t!(|mut state, ctx| {
            let departments = ctx.matches[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let region = ctx.matches[2].clone();
            let dataset = ctx.matches[3].clone();

            state
                .execute_once(
                    DownloadBano {
                        departments,
                        region: region.clone(),
                    },
                    &ctx,
                )
                .await
                .expect("failed to download OSM file");
            state
                .execute(IndexBano { region, dataset }, &ctx)
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
#[derive(Debug, PartialEq)]
pub struct IndexBano {
    pub region: String,
    pub dataset: String,
}

#[async_trait(?Send)]
impl Step for IndexBano {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region, dataset } = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(IndexCosmogony {
                region: region.to_string(),
                dataset: dataset.to_string(),
            })
            .expect("You must index admins before indexing addresses");

        bano::index_addresses(client, region, dataset, false)
            .await
            .map(|status| status.into())
            .context(error::IndexBano)
    }
}
