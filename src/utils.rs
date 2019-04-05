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

pub fn format_international_poi_label(
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

fn cosmo_to_addr_formatter_type(
    cosmo_type: &Option<cosmogony::ZoneType>,
) -> Option<address_formatter::Component> {
    use address_formatter::Component;
    match cosmo_type {
        Some(cosmogony::ZoneType::City) => Some(Component::City),
        Some(cosmogony::ZoneType::Country) => Some(Component::Country),
        Some(cosmogony::ZoneType::State) => Some(Component::State),
        Some(cosmogony::ZoneType::Suburb) => Some(Component::Suburb),
        // not sure, but it seems a cosmogony::StateDistrict is a County in address_formatter
        Some(cosmogony::ZoneType::StateDistrict) => Some(Component::County),
        _ => None,
    }
}

pub struct FormatPlaceHolder {
    street: String,
    zip_code: String,
    house_number: Option<String>,
}

impl FormatPlaceHolder {
    pub fn from_addr(house_number: String, street: String, zip_code: String) -> Self {
        Self {
            street,
            zip_code,
            house_number: Some(house_number),
        }
    }
    pub fn from_street(street: String, zip_code: String) -> Self {
        Self {
            street,
            zip_code,
            house_number: None,
        }
    }

    pub fn into_place<'b>(
        self,
        admins: impl Iterator<Item = &'b mimir::Admin>,
    ) -> address_formatter::Place {
        use address_formatter::Component;
        let mut place = address_formatter::Place::default();
        place[Component::HouseNumber] = self.house_number;
        place[Component::Road] = Some(self.street);
        place[Component::Postcode] = Some(self.zip_code);
        for a in admins {
            if let Some(addr_equivalent) = cosmo_to_addr_formatter_type(&a.zone_type) {
                place[addr_equivalent] = Some(a.name.clone());
            }
            // read country code
            if let Some(country_code) = a.codes.iter().find(|c| c.name == "ISO3166-1:alpha2") {
                place[Component::CountryCode] = Some(country_code.value.to_uppercase());
            }
        }
        place
    }
}

fn format<'a>(
    place: FormatPlaceHolder,
    admins: impl Iterator<Item = &'a mimir::Admin>,
    country_code: Option<&str>,
) -> String {
    address_formatter::FORMATTER
        .format_with_config(
            place.into_place(admins),
            address_formatter::Configuration {
                country_code: country_code.map(|s|s.to_owned()),
                ..Default::default()
            },
        )
        .map_err(|e| warn!("impossible to format label: {}", e))
        .unwrap_or_else(|_| "".to_owned())
}

fn mimir_label(lbl: String) -> String {
    // first impl is very simple, we just make the multi line address_formatting label a one liner
    lbl.trim_end().replace("\n", " ")
}

pub fn get_label<'a>(
    place: FormatPlaceHolder,
    admins: impl Iterator<Item = &'a mimir::Admin>,
    country_code: Option<&str>,
) -> String {
    mimir_label(format(place, admins, country_code))
}

pub fn get_name_and_label<'a>(
    place: FormatPlaceHolder,
    admins: impl Iterator<Item = &'a mimir::Admin>,
    country_code: Option<&str>,
) -> (String, String) {
    // first implem is very simple, the name is the first line of the label
    let lbl = format(place, admins, country_code);

    let name = lbl
        .split('\n')
        .next()
        .map(|s| s.to_string())
        .unwrap_or_else(|| lbl.clone());

    (name, mimir_label(lbl))
}
