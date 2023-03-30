// Copyright © 2016, Hove and/or its affiliates. All rights reserved.
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
    bano2mimir::{run, Error},
    settings::bano2mimir as settings,
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
    use super::*;
    use futures::TryStreamExt;
    use mimir::{
        adapters::secondary::elasticsearch::{remote, ElasticsearchStorageConfig},
        domain::ports::{primary::list_documents::ListDocuments, secondary::remote::Remote},
        utils::docker,
    };
    use mimirsbrunn::settings::{admin_settings::AdminFromCosmogonyFile, bano2mimir as settings};
    use places::addr::Addr;
    use serial_test::serial;
    use std::path::PathBuf;

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_bano_file() {
        let bano_file_path = [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "sample-bano",
            "sample-bano.csv",
        ]
        .iter()
        .collect();
        assert_correctly_index_bano(bano_file_path).await;
    }

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_bano_folder() {
        let bano_folder_path = [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "sample-bano",
        ]
        .iter()
        .collect();
        assert_correctly_index_bano(bano_folder_path).await;
    }

    async fn assert_correctly_index_bano(bano_path: PathBuf) {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");
        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: bano_path,
            cmd: settings::Command::Run,
        };

        let mut settings = settings::Settings::new(&opts).unwrap();
        let cosmogony_file: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "fixtures",
            "cosmogony",
            "ile-de-france",
            "ile-de-france.jsonl.gz",
        ]
        .iter()
        .collect();

        settings.admins = Some(AdminFromCosmogonyFile {
            french_id_retrocompatibility: false,
            langs: vec!["fr".to_string()],
            cosmogony_file,
        });
        let _res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;
        assert!(_res.is_ok());

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
            .await
            .expect("Elasticsearch Connection Established");

        let addresses: Vec<Addr> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(addresses.len(), 35);

        let addr1 = addresses
            .iter()
            .find(|&addr| addr.name == "10 Place de la Mairie")
            .unwrap();

        assert_eq!(addr1.id, "addr:1.378886;43.668175:10");

        let addr2 = addresses
            .iter()
            .find(|&addr| addr.name == "999 Rue Foncet")
            .unwrap();

        assert_eq!(addr2.zip_codes, vec!["06000", "06100", "06200", "06300"]);
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
