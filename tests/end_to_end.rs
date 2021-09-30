use cucumber::{Context, Cucumber};
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use mimir2::utils::docker;

mod error;
mod state;
mod steps;
mod utils;

#[tokio::main]
async fn main() {
    let _guard = docker::initialize()
        .await
        .expect("elasticsearch docker initialization");

    let pool = connection_test_pool().await.unwrap();

    Cucumber::<state::State>::new()
        // Specifies where our feature files exist
        .features(&["./features/admin", "./features/addresses"])
        // Adds the implementation of our steps to the runner
        .steps(steps::download::steps())
        .steps(steps::admin::steps())
        .steps(steps::address::steps())
        .steps(steps::search::steps())
        // Add some global context for all the tests, like databases.
        .context(Context::new().add(pool))
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
