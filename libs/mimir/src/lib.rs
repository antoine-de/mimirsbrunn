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

#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate chrono;
extern crate hyper;
extern crate rs_es;
extern crate regex;
extern crate geo;
extern crate geojson;

pub mod objects;
pub mod rubber;
pub use objects::*;
use chrono::Local;

pub fn logger_init() -> Result<(), log::SetLoggerError> {
    let mut builder = env_logger::LogBuilder::new();

    builder.filter(None, log::LogLevelFilter::Info)
        .format(|record| {
            format!("[{time}]{lvl}:{loc}: {msg}",
                    time = Local::now(),
                    lvl = record.level(),
                    loc = record.location().module_path(),
                    msg = record.args())
        });
    if let Ok(s) = std::env::var("RUST_LOG") {
        builder.parse(&s);
    }
    builder.init()
}
