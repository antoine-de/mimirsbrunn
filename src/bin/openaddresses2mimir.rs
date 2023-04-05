// Copyright © 2018, Hove and/or its affiliates. All rights reserved.
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

use clap::Parser;

use mimirsbrunn::{
    openaddresses2mimir::{run, Error},
    settings::openaddresses2mimir as settings,
};

fn main() -> Result<(), Error> {
    let opts = settings::Opts::parse();
    let settings = settings::Settings::new(&opts).map_err(|e| Error::Settings { source: e })?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::launch_with_runtime(
            settings.nb_threads,
            run(opts, settings),
        )
        .map_err(|e| Error::Execution { source: e }),
        settings::Command::Config => {
            println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use common::document::ContainerDocument;
    use futures::TryStreamExt;
    use mimir::{
        adapters::{
            primary::bragi::api::DEFAULT_LIMIT_RESULT_ES,
            secondary::elasticsearch::{remote, ElasticsearchStorageConfig},
        },
        domain::{
            model::{configuration::root_doctype, query::Query},
            ports::{
                primary::{list_documents::ListDocuments, search_documents::SearchDocuments},
                secondary::remote::Remote,
            },
        },
        utils::docker,
    };
    use mimirsbrunn::settings::admin_settings::AdminFromCosmogonyFile;
    use places::{addr::Addr, Place};
    use serial_test::serial;
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_oa_file() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: [
                env!("CARGO_MANIFEST_DIR"),
                "tests",
                "fixtures",
                "sample-oa.csv",
            ]
            .iter()
            .collect(),
            cmd: settings::Command::Run,
        };

        let mut settings = settings::Settings::new(&opts).unwrap();
        let cosmogony_file: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "cosmogony",
            "corse.jsonl.gz",
        ]
        .iter()
        .collect();

        settings.admins = Some(AdminFromCosmogonyFile {
            french_id_retrocompatibility: false,
            langs: vec!["fr".to_string()],
            cosmogony_file,
        });
        let res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;
        assert!(res.is_ok());

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
            .await
            .expect("Elasticsearch Connection Established");

        let search = |query: &str| {
            let client = client.clone();
            let query: String = query.into();
            async move {
                client
                    .search_documents(
                        vec![root_doctype(Addr::static_doc_type())],
                        Query::QueryString(format!("label:({})", query)),
                        DEFAULT_LIMIT_RESULT_ES,
                        None,
                    )
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|json| serde_json::from_value::<Place>(json).unwrap())
                    .map(|place| match place {
                        Place::Addr(addr) => addr,
                        _ => panic!("should only have admins"),
                    })
                    .collect::<Vec<Addr>>()
            }
        };

        let addresses: Vec<Addr> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(addresses.len(), 11);

        let results = search("Otto-Braun-Straße 72").await;
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.id, "addr:13.41931;52.52354:72");

        // We look for postcode 11111 which should have been filtered since the street name is empty
        let results = search("11111").await;
        assert_eq!(results.len(), 0);

        // Check that addresses containing multiple postcodes are read correctly
        let results = search("Rue Foncet").await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].zip_codes,
            vec!["06000", "06100", "06200", "06300"]
        )
    }

    #[tokio::test]
    #[serial]
    async fn should_fail_on_invalid_path() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: "does-not-exist.csv".into(),
            cmd: settings::Command::Run,
        };

        let settings = settings::Settings::new(&opts).unwrap();
        let res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;
        assert!(res.is_err());
    }
}
