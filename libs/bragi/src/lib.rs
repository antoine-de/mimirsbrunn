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

#![cfg_attr(feature = "serde_derive", feature(rustc_macro))]

#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate rustc_serialize;
extern crate docopt;
extern crate iron;
extern crate urlencoded;
extern crate regex;

extern crate rustless;
extern crate hyper;
extern crate jsonway;
extern crate valico;
extern crate mimir;
extern crate geojson;
extern crate geo;

extern crate rs_es;
use iron::Iron;
use rustless::Application;

#[macro_use]
extern crate mdo;
#[macro_use]
extern crate log;


pub mod api;
pub mod query;
mod model;

#[derive(RustcDecodable, Debug)]
pub struct Args {
    flag_bind: String,
    flag_connection_string: String,
}

static USAGE: &'static str = "
Usage:
    bragi --help
    bragi [--bind=<address>] [--connection-string=<connection-string>]

Options:
    -h, --help            Show this message.
    -b, --bind=<addres>   adresse to bind, [default: 127.0.0.1:4000]
    -c, --connection-string=<connection-string>
                          Elasticsearch parameters, override BRAGI_ES and default to http://localhost:9200/munin
";

pub fn runserver() {
    let mut args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    if args.flag_connection_string.is_empty() {
        args.flag_connection_string = std::env::var("BRAGI_ES").ok().unwrap_or("http://localhost:9200/munin".to_string());
    }
    let api = api::ApiEndPoint{es_cnx_string: args.flag_connection_string.clone()}.root();
    let app = Application::new(api);
    println!("listening on {}", args.flag_bind);
    Iron::new(app).http(args.flag_bind.as_str()).unwrap();

}
