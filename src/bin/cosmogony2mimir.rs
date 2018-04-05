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

use mimir::rubber::Rubber;
use mimir::objects::{Admin, AdminType};
use failure::Error;
use cosmogony::{Cosmogony, Zone};
use std::cell::Cell;
use mimirsbrunn::osm_reader::admin;

trait IntoAdmin {
    fn into_admin(self) -> Admin;
}

impl IntoAdmin for Zone {
    fn into_admin(self) -> Admin {
        let insee = admin::read_insee(&self.tags).unwrap_or("");
        let zip_codes = admin::read_zip_codes(&self.tags);
        let label = self.label;
        let weight = Cell::new(0.); //TODO, what do we want ?
        let admin_type = AdminType::City; //TODO use ZoneType
        let center = self.center.map_or(mimir::Coord::default(), |c| {
            mimir::Coord::new(c.lng(), c.lat())
        });
        Admin {
            id: format!("admin:osm:{}", self.osm_id),
            insee: insee.into(),
            level: self.admin_level.unwrap_or(0),
            label: label,
            name: self.name,
            zip_codes: zip_codes,
            weight: weight,
            boundary: self.boundary,
            coord: center,
            admin_type: admin_type,
        }
    }
}

fn send_to_es<I>(admins: I, cnx_string: &str, dataset: &str) -> Result<(), Error>
where
    I: Iterator<Item = Admin>,
{
    let mut rubber = Rubber::new(cnx_string);
    let nb_admins = rubber.index(dataset, admins)?;
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

    let admins = cosmogony.zones.into_iter().map(|z| z.into_admin());

    send_to_es(admins, &args.connection_string, &args.dataset)?;

    Ok(())
}

#[derive(StructOpt, Debug)]
struct Args {
    /// cosmogony file
    #[structopt(short = "i", long = "input")]
    input: String,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string",
                default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
}

fn main() {
    mimirsbrunn::utils::launch_run(index_cosmogony);
}
