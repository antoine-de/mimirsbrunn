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
use failure::format_err;
use futures::stream::StreamExt;
use mimir2::adapters::secondary::elasticsearch::{
    self, ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ,
};
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use mimirsbrunn::bano::Bano;
use mimirsbrunn::utils::DEFAULT_NB_THREADS;
use places::addr::Addr;
use places::admin::Admin;
use slog_scope::{info, warn};
use std::io::stdin;
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Args {
    /// Bano files. Can be either a directory or a file.
    /// If this is left empty, addresses are read from standard input.
    #[structopt(short = "i", long, parse(from_os_str))]
    input: Option<PathBuf>,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long, default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Number of threads to use
    #[structopt(short = "t", long, default_value = &DEFAULT_NB_THREADS)]
    nb_threads: usize,
    /// Number of threads to use to insert into Elasticsearch. Note that Elasticsearch is not able
    /// to handle values that are too high.
    #[structopt(short = "T", long, default_value = "1")]
    nb_insert_threads: usize,
    /// If set to true, the number inside the address won't be used for the index generation,
    /// therefore, different addresses with the same position will disappear.
    #[structopt(long)]
    use_old_index_format: bool,
    #[structopt(parse(from_os_str), long)]
    mappings: Option<PathBuf>,
    #[structopt(parse(from_os_str), long)]
    settings: Option<PathBuf>,
    /// Override value of settings using syntax `key.subkey=val`
    #[structopt(name = "setting", short = "v", long)]
    override_settings: Vec<String>,
}

async fn run(args: Args) -> Result<(), mimirsbrunn::Error> {
    info!("importing bano into Mimir");

    let config = load_es_config_for::<Addr>(args.mappings, args.settings, args.override_settings)
        .map_err(|err| format_err!("could not load configuration: {}", err))?;

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

    // TODO There might be an opportunity for optimization here:
    // Lets say we're indexing a bano department.... we don't need to retrieve
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
        let use_old_index_format = args.use_old_index_format;
        move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder, use_old_index_format)
    };

    if let Some(input_path) = args.input {
        mimirsbrunn::addr_reader::import_addresses_from_input_path(
            &client, config, input_path, into_addr,
        )
        .await
    } else {
        mimirsbrunn::addr_reader::import_addresses_from_reads(
            &client,
            config,
            true,
            args.nb_threads,
            vec![stdin()],
            into_addr,
        )
        .await
    }
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(Box::new(run)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use places::addr::Addr;
    use serial_test::serial;

    fn elasticsearch_test_url() -> String {
        std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
    }

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_bano_file() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: Some("./tests/fixtures/sample-bano.csv".into()),
            connection_string: elasticsearch_test_url(),
            mappings: Some("./config/elasticsearch/addr/mappings.json".into()),
            settings: Some("./config/elasticsearch/addr/settings.json".into()),
            use_old_index_format: false,
            nb_threads: 2,
            nb_insert_threads: 2,
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(run, args).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
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
