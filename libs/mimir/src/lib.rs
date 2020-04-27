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

// #[macro_use]
// extern crate slog;
// #[macro_use]
// extern crate slog_scope;
// #[macro_use]
// extern crate failure;

pub mod objects;
pub mod rubber;

pub use crate::objects::*;
use slog::{self, o, slog_o, Drain, Never};
use std::env;

pub fn logger_init() -> (slog_scope::GlobalLoggerGuard, ()) {
    if let Ok(s) = env::var("RUST_LOG_JSON") {
        let mut drain = slog_json::Json::new(std::io::stderr())
            .add_default_keys()
            .add_key_value(o!(
                        "module" => slog::FnValue(|rinfo : &slog::Record<'_>| {
                            rinfo.module()
                        })
            ));
        if s == "pretty" {
            drain = drain.set_pretty(true);
        }
        configure_logger(drain.build().fuse())
    } else {
        configure_logger(
            slog_term::CompactFormat::new(slog_term::PlainDecorator::new(std::io::stderr()))
                .build()
                .fuse(),
        )
    }
}

fn configure_logger<T>(drain: T) -> (slog_scope::GlobalLoggerGuard, ())
where
    T: Drain<Ok = (), Err = Never> + Send + 'static,
{
    //by default we log for info
    let builder = slog_envlogger::LogBuilder::new(drain).filter(None, slog::FilterLevel::Info);
    let builder = if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s)
    } else {
        builder
    };
    let drain = slog_async::Async::new(builder.build())
        .chan_size(256)
        .build();

    let log = slog::Logger::root(drain.fuse(), slog_o!());
    let scope_guard = slog_scope::set_global_logger(log);
    slog_stdlog::init().unwrap();
    (scope_guard, ())
}
