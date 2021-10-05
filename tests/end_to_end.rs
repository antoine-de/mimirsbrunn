use cucumber::{Context, Cucumber};
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use mimir2::adapters::secondary::elasticsearch::{ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ};
use mimir2::domain::ports::secondary::remote::Remote;
use mimir2::utils::docker;

mod error;
mod state;
mod steps;
mod utils;

/// Build test context with commonly used handles.
async fn build_context() -> Context {
    let es_pool = connection_test_pool()
        .await
        .expect("could not initialize ES pool");

    let es_client = es_pool
        .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
        .await
        .expect("Could not establish connection to Elasticsearch");

    Context::new().add(es_client)
}

#[tokio::main]
async fn main() {
    let _guard = docker::initialize()
        .await
        .expect("elasticsearch docker initialization");

    Cucumber::<state::State>::new()
        // Specifies where our feature files exist
        .features(&[
            "./features/admin",
            "./features/addresses",
            "./features/stops",
        ])
        // Adds the implementation of our steps to the runner
        .steps(steps::download::steps())
        .steps(steps::admin::steps())
        .steps(steps::address::steps())
        .steps(steps::stop::steps())
        .steps(steps::search::steps())
        // Add some global context for all the tests, like databases.
        .context(build_context().await)
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
