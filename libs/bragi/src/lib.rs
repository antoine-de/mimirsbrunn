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
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
#[macro_use]
extern crate failure;
use structopt::StructOpt;

mod extractors;
mod model;
mod prometheus_middleware;
pub mod query;
mod routes;
pub mod server;

lazy_static::lazy_static! {
    static ref BRAGI_NB_THREADS: String = (8 * ::num_cpus::get()).to_string();
}

#[derive(StructOpt, Debug)]
pub struct Args {
    /// Address to bind.
    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1:4000")]
    bind: String,
    /// Elasticsearch parameters, override BRAGI_ES environment variable.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin",
        env = "BRAGI_ES"
    )]
    connection_string: String,
    /// Number of threads used to serve http requests, override BRAGI_NB_THREADS environment variable.
    #[structopt(
        short = "t",
        long = "nb-threads",
        raw(default_value = "&BRAGI_NB_THREADS"),
        env = "BRAGI_NB_THREADS"
    )]
    nb_threads: usize,
    /// Default Max timeout in ms on ES connection.
    /// This timeout is both a network timeout and a timeout given to ES.
    #[structopt(short = "e", long = "max-es-timeout", env = "BRAGI_MAX_ES_TIMEOUT")]
    max_es_timeout: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub es_cnx_string: String, //TODO create a rs-es client
    pub max_es_timeout: Option<std::time::Duration>,
}

impl From<&Args> for Context {
    fn from(args: &Args) -> Self {
        Self {
            es_cnx_string: args.connection_string.clone(),
            max_es_timeout: args.max_es_timeout.map(std::time::Duration::from_millis),
        }
    }
}
