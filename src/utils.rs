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
use crate::Error;
use futures::future::Future;
use lazy_static::lazy_static;
use slog_scope::error;
use std::collections::BTreeMap;
use std::process::exit;
use std::sync::Arc;
use structopt::StructOpt;

lazy_static! {
    pub static ref DEFAULT_NB_THREADS: String = num_cpus::get().to_string();
}

pub fn get_zip_codes_from_admins(admins: &[Arc<places::admin::Admin>]) -> Vec<String> {
    let level = admins.iter().fold(0, |level, adm| {
        if adm.level > level && !adm.zip_codes.is_empty() {
            adm.level
        } else {
            level
        }
    });
    if level == 0 {
        return vec![];
    }
    admins
        .iter()
        .filter(|adm| adm.level == level)
        .flat_map(|adm| adm.zip_codes.iter().cloned())
        .collect()
}

pub const ADMIN_MAX_WEIGHT: f64 = 1_400_000_000.; // China's population

/// normalize the admin weight for it to be in [0, 1]
pub fn normalize_admin_weight(admins: &mut [places::admin::Admin]) {
    for admin in admins {
        admin.weight = normalize_weight(admin.weight, ADMIN_MAX_WEIGHT);
    }
}

/// normalize the weight for it to be in [0, 1]
pub fn normalize_weight(weight: f64, max_weight: f64) -> f64 {
    let w = weight / max_weight;
    if w > 1. {
        1.
    } else {
        w
    }
}

pub async fn launch_async_args<O, F, Fut>(run: F, args: O) -> Result<(), Error>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    let res = if let Err(err) = run(args).await {
        for cause in err.iter_chain() {
            eprintln!("{}", cause);
        }
        Err(err)
    } else {
        Ok(())
    };

    res
}

// Ensures the logger is initialized prior to launching a function, and also making sure the logger
// is flushed at the end. Whatever is returned by the main function is forwarded out.
pub async fn wrapped_launch_async_args<O, F, Fut>(run: F, args: O) -> Result<(), Error>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    let (guard, _) = logger_init();
    let res = if let Err(err) = run(args).await {
        for cause in err.iter_chain() {
            error!("{}", cause);
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

pub async fn wrapped_launch_async<O, F, Fut>(run: F) -> Result<(), Error>
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    wrapped_launch_async_args(run, O::from_args()).await
}

pub async fn launch_async<O, F, Fut>(run: F)
where
    O: StructOpt,
    F: FnOnce(O) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    if wrapped_launch_async(run).await.is_err() {
        // we wrap the real stuff in another method to std::exit after
        // the destruction of the logger (so we won't loose any messages)
        exit(1);
    }
}

pub fn wrapped_launch_run<O, F>(run: F) -> Result<(), Error>
where
    F: FnOnce(O) -> Result<(), Error>,
    O: StructOpt,
{
    let _guard = logger_init();
    if let Err(err) = run(O::from_args()) {
        for cause in err.iter_chain() {
            error!("{}", cause);
        }
        Err(err)
    } else {
        Ok(())
    }
}

pub fn launch_run<O, F>(run: F)
where
    F: FnOnce(O) -> Result<(), Error>,
    O: StructOpt,
{
    if wrapped_launch_run(run).is_err() {
        // we wrap the real stuff in another method to std::exit after
        // the destruction of the logger (so we won't loose any messages)
        exit(1);
    }
}

pub fn get_country_code(codes: &BTreeMap<String, String>) -> Option<String> {
    codes.get("ISO3166-1:alpha2").cloned()
}

pub fn find_country_codes<'a>(
    admins: impl Iterator<Item = &'a places::admin::Admin>,
) -> Vec<String> {
    admins.filter_map(|a| get_country_code(&a.codes)).collect()
}
