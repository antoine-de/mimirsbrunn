use mimir::utils::docker;

mod error;
mod state;
mod steps;
mod utils;

#[tokio::main]
async fn main() {
    let _guard = docker::initialize_with_param(true)
        .await
        .expect("elasticsearch docker initialization");

    utils::run_cucumber(&["./features/idf"], false).await
}
