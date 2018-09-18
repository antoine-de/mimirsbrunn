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
extern crate slog;
#[macro_use]
extern crate slog_scope;

extern crate failure;
extern crate mimir;
extern crate mimirsbrunn;
#[macro_use]
extern crate structopt;

use failure::ResultExt;
use mimir::rubber::Rubber;
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::osm_reader::admin::read_administrative_regions;
use mimirsbrunn::osm_reader::make_osm_reader;
use mimirsbrunn::osm_reader::poi::{add_address, compute_poi_weight, pois, PoiConfig};
use mimirsbrunn::osm_reader::street::{compute_street_weight, streets};
use std::path::PathBuf;

#[derive(StructOpt, Debug)]
struct Args {
    /// OSM PBF file.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: PathBuf,
    /// Admin levels to keep.
    #[structopt(short = "l", long = "level")]
    level: Vec<u32>,
    /// City level to  calculate weight.
    #[structopt(short = "C", long = "city-level", default_value = "8")]
    city_level: u32,
    /// Elasticsearch parameters.
    #[structopt(
        short = "c",
        long = "connection-string",
        default_value = "http://localhost:9200/munin"
    )]
    connection_string: String,
    /// Import ways.
    #[structopt(short = "w", long = "import-way")]
    import_way: bool,
    /// Import admins.
    #[structopt(short = "a", long = "import-admin")]
    import_admin: bool,
    /// Import POIs.
    #[structopt(short = "p", long = "import-poi")]
    import_poi: bool,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// POI configuration.
    #[structopt(short = "j", long = "poi-config", parse(from_os_str))]
    poi_config: Option<PathBuf>,
}

fn run(args: Args) -> Result<(), mimirsbrunn::Error> {
    let levels = args.level.iter().cloned().collect();
    let city_level = args.city_level;

    let mut osm_reader = make_osm_reader(&args.input)?;
    debug!("creation of indexes");
    let mut rubber = Rubber::new(&args.connection_string);
    rubber.initialize_templates()?;

    info!("creating adminstrative regions");
    let admins = if args.import_admin {
        read_administrative_regions(&mut osm_reader, levels, city_level)
    } else {
        rubber.get_admins_from_dataset(&args.dataset)?
    };
    let admins_geofinder = admins.into_iter().collect::<AdminGeoFinder>();
    {
        info!("Extracting streets from osm");
        let mut streets = streets(&mut osm_reader, &admins_geofinder)?;

        info!("computing street weight");
        compute_street_weight(&mut streets);

        if args.import_way {
            info!("importing streets into Mimir");
            let nb_streets = rubber
                .index(&args.dataset, streets.into_iter())
                .with_context(|_| {
                    format!(
                        "Error occurred when requesting street number in {}",
                        args.dataset
                    )
                })?;
            info!("Nb of indexed street: {}", nb_streets);
        }
    }
    if args.import_admin {
        let nb_admins = rubber
            .index(&args.dataset, admins_geofinder.admins())
            .with_context(|_| {
                format!(
                    "Error occurred when requesting admin number in {}",
                    args.dataset
                )
            })?;
        info!("Nb of indexed admin: {}", nb_admins);
    }

    if args.import_poi {
        let matcher = match args.poi_config {
            None => PoiConfig::default(),
            Some(path) => {
                let r = std::fs::File::open(&path).with_context(|_| {
                    format!("Error while opening configuration file {:?}", path)
                })?;
                PoiConfig::from_reader(r).unwrap()
            }
        };
        info!("Extracting pois from osm");
        let mut pois = pois(&mut osm_reader, &matcher, &admins_geofinder);

        info!("computing poi weight");
        compute_poi_weight(&mut pois);

        info!("Adding addresss in poi");
        add_address(&mut pois, &mut rubber);

        info!("Importing pois into Mimir");
        let nb_pois = rubber
            .index(&args.dataset, pois.into_iter())
            .context("Importing pois into Mimir")?;

        info!("Nb of indexed pois: {}", nb_pois);
    }
    Ok(())
}

fn main() {
    mimirsbrunn::utils::launch_run(run);
}
