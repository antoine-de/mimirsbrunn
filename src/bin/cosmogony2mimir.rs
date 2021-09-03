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

use failure::{format_err, Error};
use mimir2::{
    adapters::secondary::elasticsearch::{
        self,
        configuration::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
    },
    domain::ports::secondary::remote::Remote,
};
use serde_json::json;
use std::path::PathBuf;
use structopt::StructOpt;

// #[cfg(test)]
// #[macro_use]
// extern crate approx;

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
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    #[structopt(parse(from_os_str), default_value = "./config/admin/mappings.json")]
    mappings: PathBuf,
    #[structopt(parse(from_os_str), default_value = "./config/admin/settings.json")]
    settings: PathBuf,
    /// Number of shards for the es index
    #[structopt(short = "s", long = "nb-shards")]
    nb_shards: Option<usize>,
    /// Number of replicas for the es index
    #[structopt(short = "r", long = "nb-replicas")]
    nb_replicas: Option<usize>,
    /// Languages codes, used to build i18n names and labels
    #[structopt(name = "lang", short, long)]
    langs: Vec<String>,
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

    let settings = tokio::fs::read_to_string(args.settings.clone())
        .await
        .map_err(|err| {
            format_err!(
                "could not read settings file from '{}': {}",
                args.settings.display(),
                err.to_string(),
            )
        })?;

    let mut settings: serde_json::Value = serde_json::from_str(&settings).map_err(|err| {
        format_err!(
            "could not deserialize settings file from '{}': {}",
            args.settings.display(),
            err.to_string(),
        )
    })?;

    if let Some(nb_shards) = args.nb_shards {
        settings["index"]["number_of_shards"] = json!(nb_shards);
    }
    if let Some(nb_replicas) = args.nb_replicas {
        settings["index"]["number_of_replicas"] = json!(nb_replicas);
    }

    let mappings = tokio::fs::read_to_string(args.mappings.clone())
        .await
        .map_err(|err| {
            format_err!(
                "could not read mappings file from '{}': {}",
                args.mappings.display(),
                err.to_string(),
            )
        })?;

    let mappings = serde_json::from_str(&mappings).map_err(|err| {
        format_err!(
            "could not deserialize mappings file from '{}': {}",
            args.mappings.display(),
            err.to_string(),
        )
    })?;

    let config = IndexConfiguration {
        name: args.dataset.clone(),
        parameters: IndexParameters {
            timeout: String::from("10s"),
            wait_for_active_shards: String::from("1"), // only the primary shard
        },
        settings: IndexSettings { value: settings },
        mappings: IndexMappings { value: mappings },
    };

    mimirsbrunn::admin::index_cosmogony(args.input, args.langs, config, client)
        .await
        .map_err(|err| format_err!("could not index cosmogony: {}", err.to_string(),))
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
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
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
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not connect elasticsearch pool: Connection Error: Elasticsearch Connection Error"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_path_for_mappings() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("foo"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/invalid.json"), // a file that does not exists
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not read mappings file from"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_mappings() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("foo"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./README.md"), // exists, but not json
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not deserialize mappings file from"));
    }

    #[tokio::test]
    async fn should_return_an_error_when_given_an_invalid_path_for_input() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./invalid.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
        };

        let res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("could not index cosmogony: No such file or directory"));
    }

    #[tokio::test]
    async fn should_correctly_index_a_small_cosmogony_file() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: String::from("./tests/fixtures/cosmogony/bretagne.small.jsonl.gz"),
            connection_string: elasticsearch_test_url(),
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
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
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec!["fr".into(), "en".into()],
        };

        let _res = mimirsbrunn::utils::launch_async_args(index_cosmogony, args).await;

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");

        // FIXME Maybe should get a Vec<Admin> rather than Vec<AdminDoc>
        let admins: Vec<Admin> = client
            .list_documents()
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
            dataset: String::from("dataset"),
            mappings: PathBuf::from("./config/admin/mappings.json"),
            settings: PathBuf::from("./config/admin/settings.json"),
            nb_shards: None,
            nb_replicas: None,
            langs: vec![],
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
