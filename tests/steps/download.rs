use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use futures::stream::{self, TryStreamExt};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error;
use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use tests::download;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "osm file has been downloaded for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute_once(DownloadOsm(region), &ctx)
                .await
                .expect("failed to download OSM file");

            state
        }),
    );

    steps.given_regex_async(
        "bano files have been downloaded for (.*) into (.*)",
        t!(|mut state, ctx| {
            let departments = ctx.matches[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let region = ctx.matches[2].clone();

            state
                .execute_once(
                    DownloadBano {
                        departments,
                        region,
                    },
                    &ctx,
                )
                .await
                .expect("failed to download OSM file");

            state
        }),
    );

    steps.given_regex_async(
        "ntfs file has been downloaded for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute_once(DownloadNTFS { region }, &ctx)
                .await
                .expect("failed to download NTFS file");

            state
        }),
    );

    steps
}

#[derive(PartialEq)]
pub struct DownloadOsm(pub String);

#[async_trait(?Send)]
impl Step for DownloadOsm {
    async fn execute(&mut self, _state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self(region) = self;
        download::osm(region)
            .await
            .map(|status| status.into())
            .context(error::Download)
    }
}

#[derive(PartialEq)]
pub struct DownloadBano {
    pub departments: Vec<String>,
    pub region: String,
}

#[async_trait(?Send)]
impl Step for DownloadBano {
    async fn execute(&mut self, _state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self {
            departments,
            region,
        } = self;
        download::bano(region, departments)
            .await
            .map(|status| status.into())
            .context(error::Download)
    }
}

#[derive(Debug, PartialEq)]
pub struct DownloadNTFS {
    pub region: String,
}

#[async_trait(?Send)]
impl Step for DownloadNTFS {
    async fn execute(&mut self, _state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self { region } = self;
        download::ntfs(region)
            .await
            .map(|status| status.into())
            .context(error::Download)
    }
}
