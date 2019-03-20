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

use crate::Error;
use mimir;
use std::process::exit;
use std::sync::Arc;
use structopt::StructOpt;

pub fn format_label(admins: &[Arc<mimir::Admin>], name: &str) -> String {
    match admins.iter().position(|adm| adm.is_city()) {
        Some(idx) => format!("{} ({})", name, admins[idx].name),
        None => name.to_string(),
    }
}

pub fn format_international_labels(
    admins: &[Arc<mimir::Admin>],
    poi_names: &mimir::I18nProperties,
    default_poi_name: &str,
    default_poi_label: &str,
    langs: &[String],
) -> mimir::I18nProperties {
    let labels = langs
        .iter()
        .filter_map(|ref lang| {
            let local_poi_name = poi_names.get(lang).unwrap_or(default_poi_name);
            let i18n_poi_label =
                admins
                    .iter()
                    .find(|adm| adm.is_city())
                    .map_or(local_poi_name.to_string(), |adm| {
                        let default_admin_name = &adm.name;
                        let local_admin_name = &adm.names.get(lang).unwrap_or(&default_admin_name);
                        format!("{} ({})", local_poi_name, local_admin_name)
                    });

            if i18n_poi_label == default_poi_label {
                None
            } else {
                Some(mimir::Property {
                    key: lang.to_string(),
                    value: i18n_poi_label,
                })
            }
        })
        .collect();
    mimir::I18nProperties(labels)
}

pub fn get_zip_codes_from_admins(admins: &[Arc<mimir::Admin>]) -> Vec<String> {
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
        .into_iter()
        .filter(|adm| adm.level == level)
        .flat_map(|adm| adm.zip_codes.iter().cloned())
        .collect()
}

/// normalize the admin weight for it to be in [0, 1]
pub fn normalize_admin_weight(admins: &mut [mimir::Admin]) {
    let max = admins.iter().fold(1f64, |m, a| f64::max(m, a.weight));
    for ref mut a in admins {
        a.weight = normalize_weight(a.weight, max);
    }
}

/// normalize the weight for it to be in [0, 1]
pub fn normalize_weight(weight: f64, max_weight: f64) -> f64 {
    return weight / max_weight;
}

pub fn wrapped_launch_run<O, F>(run: F) -> Result<(), Error>
where
    F: FnOnce(O) -> Result<(), Error>,
    O: StructOpt,
{
    let _guard = mimir::logger_init();
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
