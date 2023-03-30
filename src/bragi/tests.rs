// Copyright © 2023, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Hove (www.kisio.com).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io

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
