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

    utils::run_cucumber(
        &[
            "./features/admin",
            "./features/addresses",
            "./features/stops",
        ],
        true,
    )
    .await
}
