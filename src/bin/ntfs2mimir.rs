// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
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

use failure::{format_err, Error};
use slog_scope::info;
use std::path::PathBuf;
use structopt::StructOpt;

use common::config::load_es_config_for;
use mimir2::{
    adapters::secondary::elasticsearch::{self, ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ},
    domain::ports::secondary::remote::Remote,
};

#[derive(Debug, StructOpt)]
struct Args {
    /// NTFS directory.
    #[structopt(short = "i", long = "input", parse(from_os_str), default_value = ".")]
    input: PathBuf,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Elasticsearch parameters.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin"
    )]
    connection_string: String,
    #[structopt(parse(from_os_str), long)]
    mappings: Option<PathBuf>,
    #[structopt(parse(from_os_str), long)]
    settings: Option<PathBuf>,
    /// Override value of settings using `key.subkey=value` syntax
    override_settings: Vec<String>,
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(index_ntfs).await;
}

/// Uses the commandline arguments to index an ntfs directory into Elasticsearch.
async fn index_ntfs(args: Args) -> Result<(), Error> {
    info!("Launching ntfs2mimir...");

    let pool = elasticsearch::remote::connection_pool_url(&args.connection_string)
        .await
        .map_err(|err| {
            format_err!(
                "could not create elasticsearch connection pool: {}",
                err.to_string()
            )
        })?;

    let client = pool
        .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    let config = load_es_config_for::<places::stop::Stop>(
        args.mappings,
        args.settings,
        args.override_settings,
        args.dataset,
    )
    .map_err(|err| format_err!("could not load configuration: {}", err))?;

    mimirsbrunn::stops::index_ntfs(args.input, config, &client)
        .await
        .map_err(|err| format_err!("could not index ntfs: {}", err.to_string()))
}

#[cfg(test)]
mod tests {
    use futures::stream::TryStreamExt;
    use serial_test::serial;

    use super::*;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use places::stop::Stop;

    fn elasticsearch_test_url() -> String {
        std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_es_url() {
        let url = String::from("http://example.com:demo");
        let args = Args {
            input: PathBuf::from("NA"),
            connection_string: url,
            dataset: String::from("test"),
            mappings: Some("./config/elasticsearch/stop/mappings.json".into()),
            settings: Some("./config/elasticsearch/stop/settings.json".into()),
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not create elasticsearch connection pool: Invalid Elasticsearch URL"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_url_not_es() {
        let url = String::from("http://no-es.test");
        let args = Args {
            input: PathBuf::from("NA"),
            connection_string: url,
            dataset: String::from("test"),
            mappings: None,
            settings: None,
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not connect elasticsearch pool: Connection Error: Elasticsearch Connection Error"));
    }

    #[tokio::test]
    #[serial]
    async fn should_return_an_error_when_given_an_invalid_path_for_config() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("NA"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: None,
            settings: None,
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("is neither a file nor a directory, cannot read a ntfs from it"));
    }

    #[tokio::test]
    #[serial]
    async fn should_return_an_error_when_given_an_invalid_mappings() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("foo"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: Some("./tests/fixtures/config/invalid/mappings.json".into()), // exists, but not json
            settings: None,
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        assert!(dbg!(res.unwrap_err().to_string()).contains("expected value at line 1 column 1"));
    }

    #[tokio::test]
    #[serial]
    async fn should_return_an_error_when_given_an_invalid_path_for_input() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("NA"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: None,
            settings: None,
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("is neither a file nor a directory, cannot read a ntfs from it"));
    }

    #[tokio::test]
    #[serial]
    async fn should_return_an_error_when_given_an_invalid_setting_override() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("NA"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: None,
            settings: None,
            override_settings: vec!["no-value".to_string()],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("couldn't override settings"));
    }

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_a_small_ntfs_() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("./tests/fixtures/ntfs"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: None,
            settings: None,
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        // Now we query the index we just created. Since it's a small NTFS dataset with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");

        let stops: Vec<Stop> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert!(stops.iter().all(|stop| stop.id.starts_with("stop_area")));
        assert!(stops.iter().all(|stop| stop.weight != 0f64));
    }

    #[tokio::test]
    #[serial]
    async fn should_index_ntfs_with_correct_values() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: PathBuf::from("./tests/fixtures/ntfs"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("test"),
            mappings: PathBuf::from("./config/elasticsearch/stop/mappings.json").into(),
            settings: PathBuf::from("./config/elasticsearch/stop/settings.json").into(),
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_ntfs, args).await;

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");

        let stops: Vec<Stop> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(stops.len(), 6);

        let stop1 = stops.iter().find(|&stop| stop.name == "Châtelet").unwrap();

        assert_eq!(stop1.id, "stop_area:CHA");
        assert_eq!(stop1.lines.len(), 1);
    }
}
