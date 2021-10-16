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

use common::config::load_es_config_for;
use futures::stream::StreamExt;
use mimir2::adapters::secondary::elasticsearch;
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use mimirsbrunn::bano::Bano;
use mimirsbrunn::settings::bano2mimir as settings;
use places::addr::Addr;
use places::admin::Admin;
use slog_scope::{info, warn};
use snafu::{ResultExt, Snafu};
use std::sync::Arc;
use structopt::StructOpt;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchPool {
        source: elasticsearch::remote::Error,
    },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir2::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },

    #[snafu(display("Configuration Error {}", source))]
    Configuration { source: common::config::Error },

    #[snafu(display("Import Error {}", source))]
    Import {
        source: mimirsbrunn::addr_reader::Error,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = settings::Opts::from_args();
    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::wrapped_launch_async(Box::new(run))
            .await
            .context(Execution),
        settings::Command::Config => {
            mimirsbrunn::utils::launch::wrapped_launch_async(Box::new(config))
                .await
                .context(Execution)
        }
    }
}

async fn config(opts: settings::Opts) -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings::Settings::new(&opts).map_err(Box::new)?;
    println!("{}", serde_json::to_string_pretty(&settings).unwrap());
    Ok(())
}

async fn run(opts: settings::Opts) -> Result<(), Box<dyn std::error::Error>> {
    info!("importing BANO into Mimir");
    let input = opts.input.clone(); // we save the input, because opts will be consumed by settings.

    let settings = &settings::Settings::new(&opts)
        .context(Settings)
        .map_err(Box::new)?;

    let pool = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .await
        .context(ElasticsearchPool)
        .map_err(Box::new)?;

    let client = pool
        .conn(
            settings.elasticsearch.timeout,
            &settings.elasticsearch.version_req,
        )
        .await
        .context(ElasticsearchConnection)
        .map_err(Box::new)?;

    // TODO There might be an opportunity for optimization here:
    // Lets say we're indexing a single bano department.... we don't need to retrieve
    // the admins for other regions!
    let into_addr = {
        let admins: Vec<Admin> = match client.list_documents().await {
            Ok(admins) => {
                admins
                    .map(|admin| admin.expect("could not parse admin"))
                    .collect()
                    .await
            }
            Err(err) => {
                warn!("administratives regions not found in es db. {:?}", err);
                Vec::new()
            }
        };

        let admins_by_insee = admins
            .iter()
            .cloned()
            .filter(|a| !a.insee.is_empty())
            .map(|mut a| {
                a.boundary = None; // to save some space we remove the admin boundary
                (a.insee.clone(), Arc::new(a))
            })
            .collect();

        let admins_geofinder = admins.into_iter().collect();
        move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder)
    };

    let config = load_es_config_for::<Addr>(
        opts.settings
            .iter()
            .filter_map(|s| {
                if s.starts_with("elasticsearch.addr") {
                    Some(s.to_string())
                } else {
                    None
                }
            })
            .collect(),
        settings.container.dataset.clone(),
    )
    .context(Configuration)
    .map_err(Box::new)?;

    mimirsbrunn::addr_reader::import_addresses_from_input_path(&client, config, input, into_addr)
        .await
        .context(Import)
        .map_err(|err| Box::new(err) as Box<dyn snafu::Error>) // TODO Investigate why the need to cast?
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use mimirsbrunn::settings::bano2mimir as settings;
    use places::addr::Addr;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_bano_file() {
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
                "sample-bano.csv",
            ]
            .iter()
            .collect(),
            cmd: settings::Command::Run,
        };

        let _res = mimirsbrunn::utils::launch::launch_async_args(run, opts).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = docker::ConfigElasticsearchTesting::default();
        let pool = remote::connection_pool_url(&config.url)
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(config.timeout, &config.version_req)
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
}
