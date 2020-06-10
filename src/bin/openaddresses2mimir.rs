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
use mimirsbrunn::addr_reader::{import_addresses_from_files, import_addresses_from_streams};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::{labels, utils};
use serde::{Deserialize, Serialize};
use slog_scope::{info, warn};
use std::io::stdin;
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

lazy_static! {
    static ref DEFAULT_NB_THREADS: String = num_cpus::get().to_string();
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct OpenAddress {
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

impl OpenAddress {
    pub fn into_addr(
        self,
        admins_geofinder: &AdminGeoFinder,
        use_old_index_format: bool,
        id_precision: usize,
    ) -> Result<mimir::Addr, mimirsbrunn::Error> {
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
            weight,
            zip_codes: vec![self.postcode.clone()],
            coord,
            approx_coord: None,
            distance: None,
            country_codes: country_codes.clone(),
            context: None,
        };

        let id_suffix = {
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
        };

        let id = {
            if id_precision > 0 {
                format!(
                    "addr:{:.precision$};{:.precision$}{}",
                    self.lon,
                    self.lat,
                    id_suffix,
                    precision = id_precision
                )
            } else {
                format!("addr:{};{}{}", self.lon, self.lat, id_suffix)
            }
        };

        Ok(mimir::Addr {
            id,
            name: addr_name,
            label: addr_label,
            house_number: self.number,
            street,
            coord,
            approx_coord: Some(coord.into()),
            weight,
            zip_codes: vec![self.postcode],
            distance: None,
            country_codes,
            context: None,
        })
    }
}

#[derive(StructOpt, Debug)]
struct Args {
    /// OpenAddresses files. Can be either a directory or a file.
    /// If this is left empty, addresses are read from standard input.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: Option<PathBuf>,
    /// Float precision for coordinates used to define the `id` field of addresses.
    /// Set to 0 to use exact coordinates.
    #[structopt(short = "p", long = "id-precision", default_value = "6")]
    id_precision: usize,
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
        default_value = &DEFAULT_NB_THREADS
    )]
    nb_threads: usize,
    /// Number of threads to use to insert into Elasticsearch. Note that Elasticsearch is not able
    /// to handle values that are too high.
    #[structopt(short = "T", long = "nb-insert-threads", default_value = "1")]
    nb_insert_threads: usize,
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

    let mut rubber =
        Rubber::new(&args.connection_string).with_nb_insert_threads(args.nb_insert_threads);

    let index_settings = IndexSettings {
        nb_shards: args.nb_shards,
        nb_replicas: args.nb_replicas,
    };

    // Fetch and index admins for `into_addr`
    let into_addr = {
        let admins = rubber.get_all_admins().unwrap_or_else(|err| {
            warn!(
                "Administratives regions not found in es db for dataset {}. (error: {})",
                &args.dataset, err
            );
            vec![]
        });

        let admins_geofinder = admins.into_iter().collect();
        let use_old_index_format = args.use_old_index_format;
        let id_precision = args.id_precision;

        move |a: OpenAddress| a.into_addr(&admins_geofinder, use_old_index_format, id_precision)
    };

    if let Some(input_path) = args.input {
        // Import from file(s)
        if input_path.is_dir() {
            let paths = walkdir::WalkDir::new(&input_path);
            let path_iter = paths
                .into_iter()
                .map(|p| p.unwrap().into_path())
                .filter(|p| {
                    let f = p
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.ends_with(".csv") || name.ends_with(".csv.gz"))
                        .unwrap_or(false);
                    if !f {
                        info!("skipping file {} as it is not a csv", p.display());
                    }
                    f
                });

            import_addresses_from_files(
                &mut rubber,
                true,
                args.nb_threads,
                index_settings,
                &args.dataset,
                path_iter,
                into_addr,
            )
        } else {
            import_addresses_from_files(
                &mut rubber,
                true,
                args.nb_threads,
                index_settings,
                &args.dataset,
                std::iter::once(input_path),
                into_addr,
            )
        }
    } else {
        // Import from stdin
        import_addresses_from_streams(
            &mut rubber,
            true,
            args.nb_threads,
            index_settings,
            &args.dataset,
            std::iter::once(stdin()),
            into_addr,
        )
    }
}

fn main() {
    mimirsbrunn::utils::launch_run(run);
}
