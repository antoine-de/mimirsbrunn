// Copyright © 2018, Canal TP and/or its affiliates. All rights reserved.
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

use lazy_static::lazy_static;
use mimir::rubber::{IndexSettings, Rubber};
use mimirsbrunn::addr_reader::import_addresses;
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::{labels, utils};
use serde::{Deserialize, Serialize};
use slog_scope::{info, warn};
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

lazy_static! {
    static ref DEFAULT_NB_THREADS: String = num_cpus::get().to_string();
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct OpenAddresse {
    pub id: String,
    pub street: String,
    pub postcode: String,
    pub district: String,
    pub region: String,
    pub city: String,
    pub number: String,
    pub unit: String,
    pub lat: f64,
    pub lon: f64,
}

impl OpenAddresse {
    pub fn into_addr(
        self,
        admins_geofinder: &AdminGeoFinder,
        use_old_index_format: bool,
    ) -> mimir::Addr {
        let street_id = format!("street:{}", self.id); // TODO check if thats ok
        let admins = admins_geofinder.get(&geo::Coordinate {
            x: self.lon,
            y: self.lat,
        });
        let country_codes = utils::find_country_codes(admins.iter().map(|a| a.deref()));

        let weight = admins.iter().find(|a| a.is_city()).map_or(0., |a| a.weight);
        // Note: for openaddress, we don't trust the admin hierarchy much (compared to bano)
        // so we use for the label the admins that we find in the DB
        let street_label = labels::format_street_label(
            &self.street,
            admins.iter().map(|a| a.deref()),
            &country_codes,
        );
        let (addr_name, addr_label) = labels::format_addr_name_and_label(
            &self.number,
            &self.street,
            admins.iter().map(|a| a.deref()),
            &country_codes,
        );

        let coord = mimir::Coord::new(self.lon, self.lat);
        let street = mimir::Street {
            id: street_id,
            name: self.street,
            label: street_label,
            administrative_regions: admins,
            weight: weight,
            zip_codes: vec![self.postcode.clone()],
            coord: coord.clone(),
            approx_coord: None,
            distance: None,
            country_codes: country_codes.clone(),
            context: None,
        };

        mimir::Addr {
            id: format!(
                "addr:{};{}{}",
                self.lon,
                self.lat,
                if use_old_index_format {
                    String::new()
                } else {
                    format!(
                        ":{}",
                        self.number
                            .replace(" ", "")
                            .replace("\t", "")
                            .replace("\r", "")
                            .replace("\n", "")
                            .replace("/", "-")
                            .replace(".", "-")
                            .replace(":", "-")
                            .replace(";", "-")
                    )
                }
            ),
            name: addr_name,
            label: addr_label,
            house_number: self.number,
            street: street,
            coord: coord.clone(),
            approx_coord: Some(coord.into()),
            weight: weight,
            zip_codes: vec![self.postcode],
            distance: None,
            country_codes,
            context: None,
        }
    }
}

fn index_oa<I>(
    cnx_string: &str,
    dataset: &str,
    index_settings: IndexSettings,
    files: I,
    nb_threads: usize,
    use_old_index_format: bool,
) -> Result<(), mimirsbrunn::Error>
where
    I: Iterator<Item = std::path::PathBuf>,
{
    let mut rubber = Rubber::new(cnx_string);

    let admins = rubber.get_all_admins().unwrap_or_else(|err| {
        warn!(
            "Administratives regions not found in es db for dataset {}. (error: {})",
            dataset, err
        );
        vec![]
    });
    let admins_geofinder = admins.into_iter().collect();

    import_addresses(
        &mut rubber,
        true,
        nb_threads,
        index_settings,
        dataset,
        files,
        move |a: OpenAddresse| a.into_addr(&admins_geofinder, use_old_index_format),
    )
}

#[derive(StructOpt, Debug)]
struct Args {
    /// openaddresses files. Can be either a directory or a file.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: PathBuf,
    /// Elasticsearch parameters.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin"
    )]
    connection_string: String,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Deprecated option.
    #[structopt(short = "C", long = "city-level")]
    city_level: Option<String>,
    /// Number of threads to use
    #[structopt(
        short = "t",
        long = "nb-threads",
        raw(default_value = "&DEFAULT_NB_THREADS")
    )]
    nb_threads: usize,
    /// Number of shards for the es index
    #[structopt(short = "s", long = "nb-shards", default_value = "5")]
    nb_shards: usize,
    /// Number of replicas for the es index
    #[structopt(short = "r", long = "nb-replicas", default_value = "1")]
    nb_replicas: usize,
    /// If set to true, the number inside the address won't be used for the index generation,
    /// therefore, different addresses with the same position will disappear.
    #[structopt(long = "use-old-index-format")]
    use_old_index_format: bool,
}

fn run(args: Args) -> Result<(), failure::Error> {
    info!("importing open addresses into Mimir");

    if args.city_level.is_some() {
        warn!("city-level option is deprecated, it now has no effect.");
    }

    let index_settings = IndexSettings {
        nb_shards: args.nb_shards,
        nb_replicas: args.nb_replicas,
    };
    if args.input.is_dir() {
        let paths = walkdir::WalkDir::new(&args.input);
        let path_iter = paths
            .into_iter()
            .map(|p| p.unwrap().into_path())
            .filter(|p| {
                let f = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "csv")
                    .unwrap_or(false);
                if !f {
                    info!("skipping file {} as it is not a csv", p.display());
                }
                f
            });
        index_oa(
            &args.connection_string,
            &args.dataset,
            index_settings,
            path_iter,
            args.nb_threads,
            args.use_old_index_format,
        )
    } else {
        index_oa(
            &args.connection_string,
            &args.dataset,
            index_settings,
            std::iter::once(args.input),
            args.nb_threads,
            args.use_old_index_format,
        )
    }
}

fn main() {
    mimirsbrunn::utils::launch_run(run);
}
