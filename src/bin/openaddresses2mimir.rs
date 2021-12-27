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

use clap::Parser;
use futures::stream::StreamExt;
use mimir::domain::ports::primary::generate_index::GenerateIndex;
use snafu::{ResultExt, Snafu};
use tracing::{info, warn};

use mimir::adapters::secondary::elasticsearch;
use mimir::domain::ports::primary::list_documents::ListDocuments;
use mimir::domain::ports::secondary::remote::Remote;
use mimirsbrunn::addr_reader::import_addresses_from_input_path;
use mimirsbrunn::openaddresses::OpenAddress;
use mimirsbrunn::settings::openaddresses2mimir as settings;
use places::admin::Admin;

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

    #[snafu(display("Index Creation Error {}", source))]
    IndexCreation {
        source: mimir::domain::model::error::Error,
    },
}

fn main() -> Result<(), Error> {
    let opts = settings::Opts::parse();
    let settings = settings::Settings::new(&opts).context(Settings)?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::launch_with_runtime(
            &settings.logging.path.clone(),
            settings.nb_threads,
            run(opts, settings),
        )
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
    info!("importing open addresses into Mimir");

    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch)
        .await
        .context(ElasticsearchConnection)
        .map_err(Box::new)?;

    // Fetch and index admins for `into_addr`
    let into_addr = {
        let admins: Vec<Admin> = match client.list_documents().await {
            Ok(stream) => {
                stream
                    .map(|admin| admin.expect("could not parse admin"))
                    .collect()
                    .await
            }
            Err(err) => {
                warn!("administratives regions not found in es db. {:?}", err);
                Vec::new()
            }
        };
        let admins_geofinder = admins.into_iter().collect();
        let id_precision = settings.coordinates.id_precision;
        move |a: OpenAddress| a.into_addr(&admins_geofinder, id_precision)
    };

    let addresses = import_addresses_from_input_path(opts.input, true, into_addr);

    client
        .generate_index(&settings.container, addresses)
        .await
        .context(IndexCreation)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::TryStreamExt;
    use mimir::domain::model::configuration::root_doctype;
    use serial_test::serial;

    use common::document::ContainerDocument;
    use mimir::adapters::primary::bragi::api::DEFAULT_LIMIT_RESULT_ES;
    use mimir::adapters::secondary::elasticsearch::{remote, ElasticsearchStorageConfig};
    use mimir::domain::model::query::Query;
    use mimir::domain::ports::primary::list_documents::ListDocuments;
    use mimir::domain::ports::primary::search_documents::SearchDocuments;
    use mimir::utils::docker;
    use places::{addr::Addr, Place};

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

        let settings = settings::Settings::new(&opts).unwrap();
        let _res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;

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
}
