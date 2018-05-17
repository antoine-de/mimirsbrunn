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

extern crate geo;
extern crate geojson;
extern crate iron;
#[macro_use]
extern crate lazy_static;
extern crate mimir;
extern crate rs_es;
extern crate rustless;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate heck;
extern crate urlencoded;
extern crate valico;

#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;

use iron::prelude::Chain;
use iron::Iron;
use rustless::Application;
use structopt::StructOpt;

extern crate logger;

#[macro_use]
extern crate prometheus;

extern crate hyper;

pub mod api;
mod model;
mod params;
pub mod query;
use logger::Logger;

lazy_static! {
    static ref BRAGI_ES: String =
        std::env::var("BRAGI_ES").unwrap_or_else(|_| "http://localhost:9200/munin".into());
}

#[derive(StructOpt, Debug)]
pub struct Args {
    /// Address to bind.
    #[structopt(short = "b", long = "bind", default_value = "127.0.0.1:4000")]
    bind: String,
    /// Elasticsearch parameters, override BRAGI_ES environment variable.
    #[structopt(short = "c", long = "connection-string", raw(default_value = "&BRAGI_ES"))]
    connection_string: String,
}

pub fn runserver() {
    let args = Args::from_args();
    let api = api::ApiEndPoint { es_cnx_string: args.connection_string }.root();
    let app = Application::new(api);

    let (logger_before, logger_after) = Logger::new(None);

    let mut chain = Chain::new(app);
    // Link logger_before as your first before middleware.
    chain.link_before(logger_before);

    // Link logger_after as your *last* after middleware.
    chain.link_after(logger_after);

    println!("listening on {}", args.bind);
    Iron::new(chain).http(args.bind.as_str()).unwrap();
}
