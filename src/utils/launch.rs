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

use crate::logger::logger_init;
use futures::future::Future;
use lazy_static::lazy_static;
use std::process::exit;
use structopt::StructOpt;
use tracing::error;

lazy_static! {
    pub static ref DEFAULT_NB_THREADS: String = num_cpus::get().to_string();
}

pub async fn launch_async_args<O, F, Fut>(run: F, args: O) -> Result<(), Box<dyn std::error::Error>>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let res = if let Err(err) = run(args).await {
        // To revisit when rust #58520 is resolved
        // for cause in err.chain() {
        //     eprintln!("{}", cause);
        // }
        if let Some(source) = err.source() {
            eprintln!("{}", source);
        }
        Err(err)
    } else {
        Ok(())
    };

    res
}

// Ensures the logger is initialized prior to launching a function, and also making sure the logger
// is flushed at the end. Whatever is returned by the main function is forwarded out.
pub async fn wrapped_launch_async_args<O, F, Fut>(
    run: F,
    args: O,
) -> Result<(), Box<dyn std::error::Error>>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let (guard, _) = logger_init();
    let res = if let Err(err) = run(args).await {
        // To revisit when rust #58520 is resolved
        // for cause in err.chain() {
        //     error!("{}", cause);
        // }
        if let Some(source) = err.source() {
            error!("{}", source);
        }
        Err(err)
    } else {
        Ok(())
    };

    // Ensure the logger persists until the future is resolved
    // and is flushed before the process exits.
    drop(guard);
    res
}

pub async fn wrapped_launch_async<O, F, Fut>(run: F) -> Result<(), Box<dyn std::error::Error>>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    wrapped_launch_async_args(run, O::from_args()).await
}

pub async fn launch_async<O, F, Fut>(run: F)
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    if wrapped_launch_async(run).await.is_err() {
        // we wrap the real stuff in another method to std::exit after
        // the destruction of the logger (so we won't loose any messages)
        exit(1);
    }
}

pub fn wrapped_launch_run<O, F>(run: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(O) -> Result<(), Box<dyn std::error::Error>>,
    O: StructOpt,
{
    let _guard = logger_init();
    if let Err(err) = run(O::from_args()) {
        // To revisit when rust #58520 is resolved
        // for cause in err.chain() {
        //     error!("{}", cause);
        // }
        if let Some(source) = err.source() {
            error!("{}", source);
        }
        Err(err)
    } else {
        Ok(())
    }
}

pub fn launch_run<O, F>(run: F)
where
    F: FnOnce(O) -> Result<(), Box<dyn std::error::Error>>,
    O: StructOpt,
{
    if wrapped_launch_run(run).is_err() {
        // we wrap the real stuff in another method to std::exit after
        // the destruction of the logger (so we won't loose any messages)
        exit(1);
    }
}
