use super::*;
use mimir::utils::docker;
use serial_test::serial;
use std::time::Duration;
use test_log::test;
use tokio::select;

async fn retry_until<T, F>(future_builder: impl Fn() -> F, until: impl Fn(&T) -> bool) -> T
where
    F: std::future::Future<Output = T>,
{
    let timeout = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout);
    let mut retry_interval = tokio::time::interval(Duration::from_millis(100));
    loop {
        retry_interval.tick().await;
        select! {
            response = future_builder() => {
                if until(&response) {
                    return response;
                }
            }
            _ = &mut timeout => {
                panic!("bragi did not start correctly");
            }
        }
    }
}

async fn start_bragi() {
    docker::initialize()
        .await
        .expect("elasticsearch docker initialization");

    let opts = settings::Opts {
        config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
        run_mode: Some("testing".to_string()),
        settings: vec![],
        cmd: settings::Command::Run,
    };

    let settings = settings::Settings::new(&opts).unwrap();
    let runtime_handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _ = runtime_handle.block_on(server::run_server(settings));
    });
    let _ = retry_until(
        || reqwest::get("http://localhost:5000/api/v1/status"),
        |r| {
            if let Ok(response) = r {
                response.status() == reqwest::StatusCode::OK
            } else {
                false
            }
        },
    )
    .await;
}

#[serial]
#[test(tokio::test)]
async fn status() {
    start_bragi().await;

    let response = reqwest::get("http://localhost:5000/api/v1/status")
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(
        body.get("bragi")
            .unwrap()
            .get("version")
            .unwrap()
            .as_str()
            .unwrap(),
        env!("CARGO_PKG_VERSION"),
    );
    assert_eq!(
        body.get("mimir")
            .unwrap()
            .get("version")
            .unwrap()
            .as_str()
            .unwrap(),
        env!("CARGO_PKG_VERSION"),
    );
    assert_eq!(
        body.get("elasticsearch")
            .unwrap()
            .get("health")
            .unwrap()
            .as_str()
            .unwrap(),
        "ok"
    );
}
