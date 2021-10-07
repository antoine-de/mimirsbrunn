use mimir2::utils::docker;

mod error;
mod state;
mod steps;
mod utils;

#[tokio::main]
async fn main() {
    let _guard = docker::initialize_with_param(false)
        .await
        .expect("elasticsearch docker initialization");

    utils::run_cucumber(&["./features/idf"], false).await
}
