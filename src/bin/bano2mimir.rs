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

use failure::format_err;
use lazy_static::lazy_static;
use mimir::rubber::Rubber;
use mimir2::{
    adapters::secondary::elasticsearch::{
        self,
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
    },
    domain::ports::remote::Remote,
};
use mimirsbrunn::bano::Bano;
use slog_scope::{info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;

lazy_static! {
    static ref DEFAULT_NB_THREADS: String = num_cpus::get().to_string();
}

#[derive(StructOpt, Debug)]
struct Args {
    /// Bano files. Can be either a directory or a file.
    /// If this is left empty, addresses are read from standard input.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: Option<PathBuf>,
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
    /// Number of threads to use
    #[structopt(
        short = "t",
        long = "nb-threads",
        default_value = &DEFAULT_NB_THREADS
    )]
    nb_threads: usize,
    /// Number of threads to use to insert into Elasticsearch. Note that Elasticsearch is not able
    /// to handle values that are too high.
    #[structopt(short = "T", long = "nb-insert-threads", default_value = "1")]
    nb_insert_threads: usize,
    /// Number of shards for the es index
    #[structopt(short = "s", long = "nb-shards", default_value = "5")]
    nb_shards: usize,
    /// Number of replicas for the es index
    #[structopt(short = "r", long = "nb-replicas", default_value = "1")]
    nb_replicas: usize,
    /// If set to true, the number inside the address won't be used for the index generation,
    /// therefore, different addresses with the same position will disappear.
    #[structopt(long = "use-old-index-format")]
    use_old_index_format: bool,
}

async fn run(args: Args) -> Result<(), mimirsbrunn::Error> {
    info!("importing bano into Mimir");

    let dataset = args.dataset;

    let pool = elasticsearch::remote::connection_pool(&args.connection_string)
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

    let config = IndexConfiguration {
        name: dataset.clone(),
        parameters: IndexParameters {
            timeout: String::from("10s"),
            wait_for_active_shards: String::from("1"), // only the primary shard
        },
        settings: IndexSettings {
            value: String::from(include_str!("../../config/addr/settings.json")),
        },
        mappings: IndexMappings {
            value: String::from(include_str!("../../config/addr/mappings.json")),
        },
    };

    // FIXME Need to deal with nb replicas and shards
    let mut rubber =
        Rubber::new(&args.connection_string).with_nb_insert_threads(args.nb_insert_threads);

    // Fetch and index admins for `into_addr`
    let into_addr = {
        let admins = rubber.get_all_admins().unwrap_or_else(|err| {
            warn!(
                "Administratives regions not found in es db for dataset {}. (error: {})",
                dataset, err
            );
            vec![]
        });

        let admins_geofinder = admins.iter().cloned().collect();

        let admins_by_insee = admins
            .into_iter()
            .filter(|a| !a.insee.is_empty())
            .map(|mut a| {
                a.boundary = None; // to save some space we remove the admin boundary
                (a.insee.clone(), Arc::new(a))
            })
            .collect();

        let use_old_index_format = args.use_old_index_format;
        move |b: Bano| b.into_addr(&admins_by_insee, &admins_geofinder, use_old_index_format)
    };

    if let Some(input_path) = args.input {
        mimirsbrunn::addr_reader::import_addresses_from_file(client, config, input_path, into_addr)
            .await
    } else {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(Box::new(run)).await;
}
