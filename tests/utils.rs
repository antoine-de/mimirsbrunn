use cucumber::{Context, Cucumber};
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::error::{self, Error};
use crate::state;
use crate::steps;
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use mimir2::adapters::secondary::elasticsearch::{ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ};
use mimir2::domain::ports::secondary::remote::Remote;

pub async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

pub async fn create_dir_if_not_exists(path: &Path) -> Result<(), Error> {
    if !file_exists(path).await {
        fs::create_dir(path).await.context(error::InvalidIO {
            details: format!("could no create directory {}", path.display()),
        })?;
    }

    Ok(())
}

pub async fn create_dir_if_not_exists_rec(path: &Path) -> Result<(), Error> {
    let mut head = PathBuf::new();

    for fragment in path {
        head.push(fragment);
        create_dir_if_not_exists(&head).await?;
    }

    Ok(())
}

/// Build test context with commonly used handles.
async fn build_context(reindex: bool) -> Context {
    let es_pool = connection_test_pool()
        .await
        .expect("could not initialize ES pool");

    let es_client = es_pool
        .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
        .await
        .expect("Could not establish connection to Elasticsearch");

    Context::new().add(es_client).add(reindex)
}

pub async fn run_cucumber(features: &[&str], reindex: bool) {
    Cucumber::<state::State>::new()
        // Specifies where our feature files exist
        .features(features)
        // Adds the implementation of our steps to the runner
        .steps(steps::download::steps())
        .steps(steps::admin::steps())
        .steps(steps::address::steps())
        .steps(steps::stop::steps())
        .steps(steps::search::steps())
        // Add some global context for all the tests, like databases.
        .context(build_context(reindex).await)
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
