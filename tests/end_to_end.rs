use mimir::utils::docker;

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
        true, // FIXME Not sure what this parameter is for
    )
    .await
}
