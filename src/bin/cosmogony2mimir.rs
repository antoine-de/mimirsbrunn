// Copyright © 2018, Canal TP and/or its affiliates. All rights reserved.
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

use common::config::config_from_args;
use common::document::ContainerDocument;
use config::Config;
use failure::{format_err, Error};
use mimir2::{adapters::secondary::elasticsearch, domain::ports::secondary::remote::Remote};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Args {
    /// cosmogony file
    #[structopt(short = "i", long = "input")]
    input: String,
    /// Elasticsearch parameters.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin"
    )]
    connection_string: String,
    #[structopt(
        parse(from_os_str),
        long = "mappings",
        default_value = "./config/admin/mappings.json"
    )]
    mappings: PathBuf,
    #[structopt(
        parse(from_os_str),
        long = "settings",
        default_value = "./config/admin/settings.json"
    )]
    settings: PathBuf,
    /// Languages codes, used to build i18n names and labels
    #[structopt(name = "lang", short, long)]
    langs: Vec<String>,
    /// Override value of settings using syntax `key.subkey=val`
    #[structopt(name = "setting", short = "v", long)]
    override_settings: Vec<String>,
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(index_cosmogony).await;
}

async fn index_cosmogony(args: Args) -> Result<(), Error> {
    let pool = elasticsearch::remote::connection_pool_url(&args.connection_string)
        .await
        .map_err(|err| {
            format_err!(
                "could not create elasticsearch connection pool: {}",
                err.to_string()
            )
        })?;

    let client = pool
        .conn()
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    let config =
        Config::builder()
            .add_source(places::admin::Admin::default_es_container_config())
            .add_source(config::File::from(args.mappings))
            .add_source(config::File::from(args.settings))
            .add_source(config_from_args(args.override_settings).map_err(|err| {
                format_err!("could not apply settings override: {}", err.to_string())
            })?)
            .build()?;

    mimirsbrunn::admin::index_cosmogony(args.input, args.langs, config, client)
        .await
        .map_err(|err| format_err!("could not index cosmogony: {}", err.to_string()))
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use futures::TryStreamExt;

    use super::*;
    use common::document::ContainerDocument;
    use mimir2::domain::model::query::Query;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::domain::ports::primary::search_documents::SearchDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use places::admin::Admin;
    use places::Place;

    fn elasticsearch_test_url() -> String {
        std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_es_url() {
        let url = String::from("http://example.com:demo");
        let args = Args {
            input: String::from("foo"),
            connection_string: url,
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not create elasticsearch connection pool: Invalid Elasticsearch URL"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_url_not_es() {
        let url = String::from("http://no-es.test");
        let args = Args {
            input: String::from("foo"),
            connection_string: url,
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not connect elasticsearch pool: Connection Error: Elasticsearch Connection Error"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_path_for_config() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("foo"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/invalid.json"), // a file that does not exists
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains(r#"configuration file "./config/invalid.json" not found"#));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_mappings() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("foo"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./tests/fixtures/config/invalid/mappings.json"), // exists, but not json
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(dbg!(res.unwrap_err().to_string()).contains("expected value at line 1 column 1"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_path_for_input() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./invalid.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not index cosmogony: No such file or directory"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_setting_override() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("foo"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec!["no-value".to_string()],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not apply settings override"));
    }

    #[tokio::test]
    async fn should_correctly_index_a_small_cosmogony_file() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./tests/fixtures/cosmogony/bretagne.small.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");

        let admins: Vec<Admin> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        drop(guard);

        assert!(admins.iter().all(|admin| admin.boundary.is_some()));
        assert!(admins.iter().all(|admin| admin.coord.is_valid()));
    }

    #[tokio::test]
    async fn should_correctly_index_cosmogony_with_langs() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./tests/fixtures/cosmogony/bretagne.small.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec!["fr".into(), "en".into()],
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");

        let admins: Vec<Admin> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        drop(guard);

        let brittany = admins.iter().find(|a| a.name == "Bretagne").unwrap();
        assert_eq!(brittany.names.get("fr"), Some("Bretagne"));
        assert_eq!(brittany.names.get("en"), Some("Brittany"));
        assert_eq!(brittany.labels.get("en"), Some("Brittany"));
    }

    #[tokio::test]
    async fn should_index_cosmogony_with_correct_values() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./tests/fixtures/cosmogony/bretagne.small.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            langs: vec![],
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        // Now we query the index we just created. Since a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");

        let admins: Vec<Admin> = client
            .search_documents(
                vec![String::from(Admin::static_doc_type())], // Fixme Use ContainerDoc::static_doc_type()
                Query::QueryString(String::from("name:bretagne")),
            )
            .await
            .unwrap()
            .into_iter()
            .map(|json| serde_json::from_value::<Place>(json).unwrap())
            .map(|place| match place {
                Place::Admin(admin) => admin,
                _ => panic!("should only have admins"),
            })
            .collect();

        drop(guard);

        let brittany = admins.iter().find(|a| a.name == "Bretagne").unwrap();
        assert_eq!(brittany.id, "admin:osm:relation:102740");
        assert_eq!(brittany.zone_type, Some(cosmogony::ZoneType::State));
        assert_relative_eq!(brittany.weight, 0.002_298, epsilon = 1e-6);
        assert_eq!(
            brittany.codes,
            vec![
                ("ISO3166-2", "FR-BRE"),
                ("ref:INSEE", "53"),
                ("ref:nuts", "FRH;FRH0"),
                ("ref:nuts:1", "FRH"),
                ("ref:nuts:2", "FRH0"),
                ("wikidata", "Q12130")
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
        )
    }
}
