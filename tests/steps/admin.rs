use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use snafu::ResultExt;

use crate::error;
use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use crate::steps::download::DownloadOsm;
use mimir::adapters::secondary::elasticsearch::ElasticsearchStorage;
use tests::cosmogony;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        r#"osm file has been processed by cosmogony for ([^\s]*)"#,
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            assert!(!region.is_empty());

            state
                .execute_once(GenerateCosmogony(region), &ctx)
                .await
                .expect("failed to generate cosmogony file");

            state
        }),
    );

    steps.given_regex_async(
        r#"cosmogony file has been indexed for ([^\s]*)(?: as (.*))?"#,
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
                .execute_once(IndexCosmogony { region, dataset }, &ctx)
                .await
                .expect("failed to index cosmogony file");

            state
        }),
    );

    // This step is a condensed format for download + generate + index
    steps.given_regex_async(
        r#"admins have been indexed for ([^\s]*)(?: as (.*))?"#,
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
                .execute_once(DownloadOsm(region.to_string()), &ctx)
                .await
                .expect("failed to download OSM");

            state
                .execute_once(GenerateCosmogony(region.to_string()), &ctx)
                .await
                .expect("failed to generate cosmogony file");

            state
                .execute_once(IndexCosmogony { region, dataset }, &ctx)
                .await
                .expect("failed to index cosmogony file");

            state
        }),
    );

    // This step is a condensed format for download + generate + index
    // FIXME This is the same code has the previous step, except the dataset is not optional.
    // The previous one is more general, so this step should probably be deleted.
    steps.given_regex_async(
        "admins have been indexed for (.*) as (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();
            let dataset = ctx.matches[2].clone();
            state
                .execute_once(DownloadOsm(region.to_string()), &ctx)
                .await
                .expect("failed to download OSM");

            state
                .execute_once(GenerateCosmogony(region.to_string()), &ctx)
                .await
                .expect("failed to generate cosmogony file");

            state
                .execute_once(IndexCosmogony { region, dataset }, &ctx)
                .await
                .expect("failed to index cosmogony file");

            state
        }),
    );

    steps
}

#[derive(PartialEq)]
pub struct GenerateCosmogony(pub String);

#[async_trait(?Send)]
impl Step for GenerateCosmogony {
    async fn execute(&mut self, state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self(region) = self;

        state
            .status_of(&DownloadOsm(region.to_string()))
            .expect("can't generate cosmogony file without downloading from OSM first");

        cosmogony::generate(region, false)
            .await
            .map(|status| status.into())
            .context(error::GenerateCosmogony)
    }
}

/// Index a cosmogony file for given region into ES.
///
/// This assumes that a cosmogony file has already been generated before.
#[derive(Debug, PartialEq)]
pub struct IndexCosmogony {
    pub region: String,
    pub dataset: String,
}

#[async_trait(?Send)]
impl Step for IndexCosmogony {
    async fn execute(&mut self, state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region, dataset } = self;
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        state
            .status_of(&GenerateCosmogony(region.to_string()))
            .expect("can't generate cosmogony file without downloading from OSM first");

        cosmogony::index_admins(client, region, dataset, false)
            .await
            .map(|status| status.into())
            .context(error::IndexCosmogony)
    }
}
