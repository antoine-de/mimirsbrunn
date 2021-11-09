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

use snafu::{ResultExt, Snafu};
use structopt::StructOpt;

use mimir::adapters::secondary::elasticsearch;
use mimir::domain::ports::secondary::remote::Remote;
use mimirsbrunn::settings::cosmogony2mimir as settings;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },

    #[snafu(display("Configuration Error {}", source))]
    Configuration { source: common::config::Error },

    #[snafu(display("Import Error {}", source))]
    Import { source: mimirsbrunn::stops::Error },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = settings::Opts::from_args();
    let settings = settings::Settings::new(&opts).context(Settings)?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::wrapped_launch_async(
            &settings.logging.path.clone(),
            move || run(opts, settings),
        )
        .await
        .context(Execution),
        settings::Command::Config => {
            println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            Ok(())
        }
    }
}

async fn run(
    opts: settings::Opts,
    settings: settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch)
        .await
        .context(ElasticsearchConnection)
        .map_err(Box::new)?;

    mimirsbrunn::stops::index_ntfs(opts.input, settings.container, &client)
        .await
        .context(Import)
        .map_err(|err| Box::new(err) as Box<dyn snafu::Error>) // TODO Investigate why the need to cast?
}

#[cfg(test)]
mod tests {
    use futures::TryStreamExt;
    use serial_test::serial;

    use super::*;
    use mimir::adapters::secondary::elasticsearch::{remote, ElasticsearchStorageConfig};
    use mimir::domain::ports::primary::list_documents::ListDocuments;
    use mimir::utils::docker;
    use places::stop::Stop;

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_a_small_ntfs_file() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(), // Not a valid config base dir
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: [
                env!("CARGO_MANIFEST_DIR"),
                "tests",
                "fixtures",
                "ntfs",
                "limousin",
            ]
            .iter()
            .collect(),
            cmd: settings::Command::Run,
        };

        let settings = settings::Settings::new(&opts).unwrap();
        let _res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
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
    }
}
