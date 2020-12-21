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

#[macro_use]
extern crate prometheus;

use mimir::rubber::Rubber;
use slog_scope::debug;
use std::convert::TryFrom;
use std::time::Duration;
use structopt::StructOpt;

use std::fs::read_to_string;

mod extractors;
mod model;
pub mod prometheus_middleware;
pub(crate) mod query;
mod query_settings;
mod routes;
pub mod server;

pub use query::make_place as query_make_place;
pub use query_settings::QuerySettings;

lazy_static::lazy_static! {
    static ref BRAGI_NB_THREADS: String = (8 * ::num_cpus::get()).to_string();
}

#[derive(StructOpt, Debug, Clone, Default)]
pub struct Args {
    /// Address to bind.
    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1:4000")]
    pub bind: String,
    /// Elasticsearch parameters, override BRAGI_ES environment variable.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin",
        env = "BRAGI_ES"
    )]
    pub connection_string: String,
    /// Number of threads used to serve http requests, override BRAGI_NB_THREADS environment variable.
    #[structopt(
        short = "t",
        long = "nb-threads",
        default_value = &BRAGI_NB_THREADS,
        env = "BRAGI_NB_THREADS"
    )]
    pub nb_threads: usize,
    /// Default Max timeout in ms on ES connection.
    /// This timeout is both a network timeout and a timeout given to ES.
    #[structopt(short = "e", long = "max-es-timeout", env = "BRAGI_MAX_ES_TIMEOUT")]
    pub max_es_timeout: Option<u64>,

    /// Custom timeout for the /reverse
    /// this is bounded by `max_es_timeout` and is used because for the moment we cannot easily change the timeout of a given rubber
    #[structopt(long = "max-es-reverse-timeout", env = "BRAGI_MAX_ES_REVERSE_TIMEOUT")]
    pub max_es_reverse_timeout: Option<u64>,
    /// Custom timeout for the /autocomplete
    /// this is bounded by `max_es_timeout` and is used because for the moment we cannot easily change the timeout of a given rubber
    #[structopt(
        long = "max-es-autocomplete-timeout",
        env = "BRAGI_MAX_ES_AUTOCOMPLETE_TIMEOUT"
    )]
    pub max_es_autocomplete_timeout: Option<u64>,
    /// Custom timeout for the /features
    /// this is bounded by `max_es_timeout` and is used because for the moment we cannot easily change the timeout of a given rubber
    #[structopt(
        long = "max-es-features-timeout",
        env = "BRAGI_MAX_ES_FEATURES_TIMEOUT"
    )]
    pub max_es_features_timeout: Option<u64>,

    /// Cache duration for http response served by bragi
    /// This only set the Cache-control Header, it doesn't enable cache on bragi side
    /// The duration is in seconds
    #[structopt(
        long = "http-cache-duration",
        env = "BRAGI_HTTP_CACHE_DURATION",
        default_value = "3600"
    )]
    pub http_cache_duration: u32,
    #[structopt(long = "weight-config-file")]
    pub weight_config_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Context {
    reverse_rubber: Rubber,
    features_rubber: Rubber,
    autocomplete_rubber: Rubber,
    pub cnx_string: String,
    pub http_cache_duration: u32,
    // pub rubber: Rubber,
    query_settings: QuerySettings,
}

impl TryFrom<&Args> for Context {
    type Error = String;

    fn try_from(args: &Args) -> Result<Self, Self::Error> {
        let max_es_timeout = args.max_es_timeout.map(Duration::from_millis);

        // the timeout is the min between the timeout set at startup time and at query time
        let bounded_timeout = |specific_timeout: Option<u64>| {
            specific_timeout
                .map(Duration::from_millis)
                .map(|t| match max_es_timeout {
                    Some(dt) => t.min(dt),
                    None => t,
                })
                .or_else(|| max_es_timeout)
        };

        let content = match args.weight_config_file {
            Some(ref file_path) => read_to_string(&file_path)
                .map_err(|e| format!("Failed to read `{}`: {}", file_path, e))?,
            None => include_str!("../../../config/bragi-settings.toml").to_owned(),
        };
        Ok(Self {
            reverse_rubber: Rubber::new_with_timeout(
                &args.connection_string,
                bounded_timeout(args.max_es_reverse_timeout),
            ),
            features_rubber: Rubber::new_with_timeout(
                &args.connection_string,
                bounded_timeout(args.max_es_features_timeout),
            ),
            autocomplete_rubber: Rubber::new_with_timeout(
                &args.connection_string,
                bounded_timeout(args.max_es_autocomplete_timeout),
            ),
            cnx_string: args.connection_string.clone(),
            http_cache_duration: args.http_cache_duration,
            query_settings: QuerySettings::new(&content).map_err(|err| {
                format!(
                    "failed to parse `{}`: {}",
                    args.weight_config_file
                        .as_deref()
                        .unwrap_or_else(|| "config/bragi-settings.toml"),
                    err
                )
            })?,
        })
    }
}

impl Context {
    pub fn get_rubber_for_reverse(&self, timeout: Option<Duration>) -> Rubber {
        clone_or_create(&self.reverse_rubber, timeout)
    }
    pub fn get_rubber_for_features(&self, timeout: Option<Duration>) -> Rubber {
        clone_or_create(&self.features_rubber, timeout)
    }
    pub fn get_rubber_for_autocomplete(&self, timeout: Option<Duration>) -> Rubber {
        clone_or_create(&self.autocomplete_rubber, timeout)
    }
    pub fn get_query_settings(&self) -> &QuerySettings {
        &self.query_settings
    }
}

fn clone_or_create(rubber: &Rubber, timeout: Option<Duration>) -> Rubber {
    if rubber.timeout == timeout {
        // we clone the rs_es_client, reusing the reqwest connection pool
        rubber.clone()
    } else {
        // if the timeout is different, since there as no easy way to change the timeout for the moment
        // we build a new Rubber (and thus a new connection)
        debug!("creating a new rubber for timeout {:?}", &timeout);
        Rubber::new_with_timeout(&rubber.cnx_string, timeout)
    }
}
