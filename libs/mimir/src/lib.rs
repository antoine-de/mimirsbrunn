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

extern crate serde;
extern crate serde_json;

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate chrono;
extern crate geo;
extern crate geojson;
extern crate hyper;
extern crate rs_es;

pub mod objects;
pub mod rubber;

pub use objects::*;
use chrono::Local;
use std::env;
use std::io::Write;

pub fn logger_init() {
    let mut builder = env_logger::Builder::new();

    if env::var("LOG_TIME").ok().map_or(false, |s| s == "1") {
        builder.format(|formater, record| {
            write!(
                formater,
                "[{time}] [{lvl}] [{loc}] {msg}\n",
                time = Local::now(),
                lvl = record.level(),
                loc = record.module_path().unwrap_or("unknown"),
                msg = record.args()
            )
        });
    } else {
        builder.format(|formater, record| {
            write!(
                formater,
                "[{lvl}] [{loc}] {msg}\n",
                lvl = record.level(),
                loc = record.module_path().unwrap_or("unknown"),
                msg = record.args()
            )
        });
    }

    builder.filter(None, log::LevelFilter::Info);
    if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s);
    }
    builder.init()
}
