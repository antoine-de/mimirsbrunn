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
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
    },
    domain::ports::remote::Remote,
};

use serde_json::json;
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

    let mut settings = json!(settings);
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

    let mappings = json!(mappings);

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
