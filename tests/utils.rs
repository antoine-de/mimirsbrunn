use cucumber::{Context, Cucumber};

use crate::state;
use crate::steps;
use mimir2::adapters::secondary::elasticsearch::remote::connection_pool_url;
use common::config::config_from;
use mimir2::domain::ports::secondary::remote::Remote;
use mimir2::utils::docker::ConfigElasticsearchTesting;

/// Build test context with commonly used handles.
async fn build_context(reindex: bool) -> Context {
    let config = ConfigElasticsearchTesting::default();

    let es_pool = connection_pool_url(&config.url)
    let config = config_from(
        &PathBuf::from("config"),
        &["elasticsearch"],
        "testing".to_string(),
        "TEST",
        None,
    )
    .unwrap();

    let es_client = es_pool
        .conn(
            config
                .get("elasticsearch")
                .expect("invalid `elasticsearch` in config"),
        )
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
        .steps(steps::street::steps())
        .steps(steps::poi::steps())
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
