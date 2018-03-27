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

extern crate failure;
extern crate mimir;
extern crate mimirsbrunn;
extern crate navitia_model;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
#[macro_use]
extern crate structopt;

use std::path::PathBuf;
use mimirsbrunn::stops::*;
use navitia_model::objects as navitia;
use navitia_model::collection::Idx;
use failure::ResultExt;

#[derive(Debug, StructOpt)]
struct Args {
    /// NTFS directory.
    #[structopt(short = "i", long = "input", parse(from_os_str), default_value = ".")]
    input: PathBuf,
    /// Name of the dataset.
    #[structopt(short = "d", long = "dataset", default_value = "fr")]
    dataset: String,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string",
                default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Deprecated option.
    #[structopt(short = "C", long = "city-level")]
    city_level: Option<String>,
}

fn to_mimir(
    idx: Idx<navitia::StopArea>,
    stop_area: &navitia::StopArea,
    navitia: &navitia_model::PtObjects,
) -> mimir::Stop {
    let commercial_modes = navitia
        .get_corresponding_from_idx(idx)
        .into_iter()
        .map(|cm_idx| mimir::CommercialMode {
            id: format!("commercial_mode:{}", navitia.commercial_modes[cm_idx].id),
            name: navitia.commercial_modes[cm_idx].name.clone(),
        })
        .collect();
    let physical_modes = navitia
        .get_corresponding_from_idx(idx)
        .into_iter()
        .map(|pm_idx| mimir::PhysicalMode {
            id: format!("physical_mode:{}", navitia.physical_modes[pm_idx].id),
            name: navitia.physical_modes[pm_idx].name.clone(),
        })
        .collect();

    let feed_publishers = navitia
        .get_corresponding_from_idx(idx)
        .into_iter()
        .map(|contrib_idx| mimir::FeedPublisher {
            id: navitia.contributors[contrib_idx].id.clone(),
            name: navitia.contributors[contrib_idx].name.clone(),
            license: navitia.contributors[contrib_idx]
                .license
                .clone()
                .unwrap_or_else(|| "".into()),
            url: navitia.contributors[contrib_idx]
                .website
                .clone()
                .unwrap_or_else(|| "".into()),
        })
        .collect();

    mimir::Stop {
        id: format!("stop_area:{}", stop_area.id),
        label: stop_area.name.clone(),
        name: stop_area.name.clone(),
        coord: mimir::Coord::new(stop_area.coord.lon, stop_area.coord.lat),
        commercial_modes: commercial_modes,
        physical_modes: physical_modes,
        administrative_regions: vec![],
        weight: 0.,
        zip_codes: vec![],
        coverages: vec![],
        timezone: stop_area.timezone.clone().unwrap_or(format!("")),
        codes: stop_area
            .codes
            .iter()
            .map(|&(ref t, ref v)| mimir::Code {
                name: t.clone(),
                value: v.clone(),
            })
            .collect(),
        properties: stop_area
            .object_properties
            .iter()
            .map(|&(ref k, ref v)| mimir::Property {
                key: k.clone(),
                value: v.clone(),
            })
            .collect(),
        feed_publishers: feed_publishers,
    }
}

fn main() {
    mimirsbrunn::utils::launch_run(run);
}
fn run(args: Args) -> Result<(), navitia_model::Error> {
    info!("Launching ntfs2mimir...");

    if args.city_level.is_some() {
        warn!("city-level option is deprecated, it now has no effect.");
    }

    let navitia = navitia_model::ntfs::read(&args.input)?;
    let nb_stop_points = navitia
        .stop_areas
        .iter()
        .map(|(idx, sa)| {
            let id = format!("stop_area:{}", sa.id);
            let nb_stop_points = navitia
                .get_corresponding_from_idx::<_, navitia::StopPoint>(idx)
                .len();
            (id, nb_stop_points as u32)
        })
        .collect();
    let mut stops: Vec<mimir::Stop> = navitia
        .stop_areas
        .iter()
        .map(|(idx, sa)| to_mimir(idx, sa, &navitia))
        .collect();
    set_weights(stops.iter_mut(), &nb_stop_points);
    import_stops(stops, &args.connection_string, &args.dataset).with_context(|_| {
        format!(
            "Error occurred when importing stops into {} on {}",
            args.dataset, args.connection_string
        )
    })?;
    Ok(())
}

#[test]
fn test_bad_connection_string() {
    let args = Args {
        input: PathBuf::from("./tests/fixtures/ntfs"),
        connection_string: "http://localhost:1".to_string(),
        dataset: "bob".to_string(),
        city_level: None,
    };
    let causes = run(args)
        .unwrap_err()
        .causes()
        .into_iter()
        .map(|cause| format!("{}", cause))
        .collect::<Vec<String>>();
    assert_eq!(
        causes,
        [
            "Error occurred when importing stops into bob on http://localhost:1".to_string(),
            "Error: Connection refused (os error 111) while creating template template_addr"
                .to_string()
        ]
    );
}

#[test]
fn test_bad_file() {
    let args = Args {
        input: PathBuf::from("./tests/fixtures/not_exist"),
        connection_string: "http://localhost:9200".to_string(),
        dataset: "bob".to_string(),
        city_level: None,
    };
    let causes = run(args)
        .unwrap_err()
        .causes()
        .into_iter()
        .map(|cause| format!("{}", cause))
        .collect::<Vec<String>>();
    assert_eq!(
        causes,
        [
            "Error reading \"./tests/fixtures/not_exist/contributors.txt\"",
            "No such file or directory (os error 2)",
        ]
    );
}
