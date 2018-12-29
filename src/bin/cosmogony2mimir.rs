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

extern crate cosmogony;
extern crate failure;
#[macro_use]
extern crate log;
extern crate mimir;
extern crate mimirsbrunn;
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate osmpbfreader;

use cosmogony::{Cosmogony, Zone, ZoneIndex};
use failure::Error;
use mimir::objects::Admin;
use mimir::rubber::{IndexSettings, Rubber};
use mimirsbrunn::osm_reader::admin;
use mimirsbrunn::osm_reader::osm_utils;
use mimirsbrunn::utils::normalize_admin_weight;
use std::collections::BTreeMap;

trait IntoAdmin {
    fn into_admin(self, _: &BTreeMap<ZoneIndex, String>) -> Admin;
}

fn get_weight(tags: &osmpbfreader::Tags, center_tags: &osmpbfreader::Tags) -> f64 {
    // to have an admin weight we use the osm 'population' tag to priorize
    // the big zones over the small one.
    // Note: this tags is not often filled , so only some zones
    // will have a weight (but the main cities have it).
    tags.get("population")
        .and_then(|p| p.parse().ok())
        .or_else(|| center_tags.get("population")?.parse().ok())
        .unwrap_or(0.)
}

impl IntoAdmin for Zone {
    fn into_admin(self, zones_osm_id: &BTreeMap<ZoneIndex, String>) -> Admin {
        let insee = admin::read_insee(&self.tags).unwrap_or("");
        let zip_codes = admin::read_zip_codes(&self.tags);
        let label = self.label;
        let weight = get_weight(&self.tags, &self.center_tags);
        let center = self.center.map_or(mimir::Coord::default(), |c| {
            mimir::Coord::new(c.lng(), c.lat())
        });
        let format_id = |id| format!("admin:osm:{}", id);
        let parent_osm_id = self
            .parent
            .and_then(|id| zones_osm_id.get(&id))
            .map(format_id);
        Admin {
            id: format_id(&self.osm_id),
            insee: insee.into(),
            level: self.admin_level.unwrap_or(0),
            label: label,
            name: self.name,
            zip_codes: zip_codes,
            weight: weight,
            bbox: self.bbox,
            boundary: self.boundary,
            coord: center,
            zone_type: self.zone_type,
            parent_id: parent_osm_id,
            codes: osm_utils::get_osm_codes_from_tags(&self.tags),
        }
    }
}

fn send_to_es(
    admins: Vec<Admin>,
    cnx_string: &str,
    dataset: &str,
    index_settings: IndexSettings,
) -> Result<(), Error> {
    let mut rubber = Rubber::new(cnx_string);
    rubber.initialize_templates()?;
    let nb_admins = rubber.index(dataset, &index_settings, admins.into_iter())?;
    info!("{} admins added.", nb_admins);
    Ok(())
}

fn load_cosmogony(input: &str) -> Result<Cosmogony, Error> {
    serde_json::from_reader(std::fs::File::open(&input)?)
        .map_err(|e| failure::err_msg(e.to_string()))
}

fn index_cosmogony(args: Args) -> Result<(), Error> {
    info!("importing cosmogony into Mimir");
    let cosmogony = load_cosmogony(&args.input)?;

    let cosmogony_id_to_osm_id: BTreeMap<_, _> = cosmogony
        .zones
        .iter()
        .map(|z| (z.id.clone(), z.osm_id.clone()))
        .collect();

    let mut admins: Vec<_> = cosmogony
        .zones
        .into_iter()
        .map(|z| z.into_admin(&cosmogony_id_to_osm_id))
        .collect();

    normalize_admin_weight(&mut admins);

    let index_settings = IndexSettings {
        nb_shards: args.nb_shards,
        nb_replicas: args.nb_replicas,
    };
    send_to_es(
        admins,
        &args.connection_string,
        &args.dataset,
        index_settings,
    )?;

    Ok(())
}

#[derive(StructOpt, Debug)]
struct Args {
    /// cosmogony file
    #[structopt(short = "i", long = "input")]
    input: String,
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
    /// Number of shards for the es index
    #[structopt(short = "s", long = "nb-shards", default_value = "1")]
    nb_shards: usize,
    /// Number of replicas for the es index
    #[structopt(short = "r", long = "nb-replicas", default_value = "1")]
    nb_replicas: usize,
}

fn main() {
    mimirsbrunn::utils::launch_run(index_cosmogony);
}
