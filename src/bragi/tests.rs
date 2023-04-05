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
use serde_json::json;
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
        body.pointer("/bragi/version").unwrap(),
        &json!(env!("CARGO_PKG_VERSION")),
    );
    assert_eq!(
        body.pointer("/mimir/version").unwrap(),
        &json!(env!("CARGO_PKG_VERSION")),
    );
    assert_eq!(body.pointer("/elasticsearch/health").unwrap(), &json!("ok"));
}

async fn cosmogony2mimir() {
    let opts = mimirsbrunn::settings::cosmogony2mimir::Opts {
        config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(), // Not a valid config base dir
        run_mode: Some("testing".to_string()),
        settings: vec![],
        input: [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "cosmogony",
            "corse.jsonl.gz",
        ]
        .iter()
        .collect(),
        cmd: mimirsbrunn::settings::cosmogony2mimir::Command::Run,
    };

    let settings = mimirsbrunn::settings::cosmogony2mimir::Settings::new(&opts).unwrap();
    let _res = mimirsbrunn::utils::launch::launch_async(move || {
        mimirsbrunn::cosmogony2mimir::run(opts, settings)
    })
    .await;
}

async fn osm2mimir() {
    let opts = mimirsbrunn::settings::osm2mimir::Opts {
        config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
        run_mode: Some("testing".to_string()),
        settings: vec![],
        input: [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "osm",
            "corse.osm.pbf",
        ]
        .iter()
        .collect(),
        cmd: mimirsbrunn::settings::osm2mimir::Command::Run,
    };
    let mut settings = mimirsbrunn::settings::osm2mimir::Settings::new(&opts).unwrap();
    settings.streets.import = true;
    let _res = mimirsbrunn::utils::launch::launch_async(move || {
        mimirsbrunn::osm2mimir::run(opts, settings)
    })
    .await;
}

async fn bano2mimir() {
    let opts = mimirsbrunn::settings::bano2mimir::Opts {
        config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
        run_mode: Some("testing".to_string()),
        settings: vec![],
        input: [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "bano",
            "corse.csv",
        ]
        .iter()
        .collect(),
        cmd: mimirsbrunn::settings::bano2mimir::Command::Run,
    };

    let settings = mimirsbrunn::settings::bano2mimir::Settings::new(&opts).unwrap();
    let _res = mimirsbrunn::utils::launch::launch_async(move || {
        mimirsbrunn::bano2mimir::run(opts, settings)
    })
    .await;
}

#[serial]
#[test(tokio::test)]
async fn autocomplete() {
    start_bragi().await;
    cosmogony2mimir().await;

    let response =
        reqwest::get("http://localhost:5000/api/v1/autocomplete?q=Propriano&type[]=city")
            .await
            .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.json::<serde_json::Value>().await.unwrap();

    let features = body.pointer("/features").unwrap();
    assert_eq!(features.as_array().unwrap().len(), 1);

    let geocoding = features.pointer("/0/properties/geocoding").unwrap();
    assert_eq!(geocoding.get("name").unwrap(), &json!("Propriano"));
    assert_eq!(geocoding.get("type").unwrap(), &json!("zone"));
    assert_eq!(geocoding.get("zone_type").unwrap(), &json!("city"));
}

#[serial]
#[test(tokio::test)]
async fn reverse() {
    start_bragi().await;
    cosmogony2mimir().await;
    osm2mimir().await;
    bano2mimir().await;

    let response =
        reqwest::get("http://localhost:5000/api/v1/reverse?lat=41.920063&lon=8.736635&limit=1")
            .await
            .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.json::<serde_json::Value>().await.unwrap();

    let features = body.pointer("/features").unwrap();
    assert_eq!(features.as_array().unwrap().len(), 1);

    let geocoding = features.pointer("/0/properties/geocoding").unwrap();
    assert_eq!(geocoding.get("housenumber").unwrap(), &json!("1"));
    assert_eq!(geocoding.get("city").unwrap(), &json!("Ajaccio"));
    assert_eq!(
        geocoding.get("label").unwrap(),
        &json!("1 Cours Napoléon (Ajaccio)")
    );
    assert_eq!(geocoding.get("postcode").unwrap(), &json!("20000"));
    assert_eq!(geocoding.get("type").unwrap(), &json!("house"));
}

#[serial]
#[test(tokio::test)]
async fn features() {
    start_bragi().await;
    cosmogony2mimir().await;

    let response = reqwest::get(
        "http://localhost:5000/api/v1/features/admin:fr:2A249?poi_dataset[]=needed-parameter-but-not-important-for-admin",
    )
    .await
    .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.json::<serde_json::Value>().await.unwrap();

    tracing::debug!("{body}");
    let features = body.pointer("/features").unwrap();
    assert_eq!(features.as_array().unwrap().len(), 1);

    let geocoding = features.pointer("/0/properties/geocoding").unwrap();
    assert_eq!(geocoding.get("id").unwrap(), &json!("admin:fr:2A249"));
    assert_eq!(geocoding.get("name").unwrap(), &json!("Propriano"));
    assert_eq!(geocoding.get("postcode").unwrap(), &json!("20110"));
    assert_eq!(geocoding.get("level").unwrap(), &json!(8));
    assert_eq!(geocoding.get("type").unwrap(), &json!("zone"));
    assert_eq!(geocoding.get("zone_type").unwrap(), &json!("city"));
}
