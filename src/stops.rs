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

use std::rc::Rc;
use mimir::rubber::{Rubber, TypedIndex};
use utils::{format_label, get_zip_codes_from_admins};
use mimir;
use admin_geofinder::AdminGeoFinder;
use std::collections::HashMap;
use std::mem::replace;

const GLOBAL_STOP_INDEX_NAME: &'static str = "munin_global_stops";

pub fn set_weights<'a, It>(stops: It, nb_stop_points: &HashMap<String, u32>)
where
    It: Iterator<Item = &'a mut mimir::Stop>,
{
    let max = *nb_stop_points.values().max().unwrap_or(&1) as f64;
    for stop in stops {
        stop.weight = if let Some(weight) = nb_stop_points.get(&stop.id) {
            *weight as f64 / max
        } else {
            0.
        };
    }
}

pub fn import_stops(mut stops: Vec<mimir::Stop>, connection_string: &str, dataset: &str) {
    info!("creation of indexes");
    let mut rubber = Rubber::new(connection_string);
    rubber.initialize_templates().unwrap();

    attach_stops_to_admins(stops.iter_mut(), &mut rubber);

    for stop in &mut stops {
        stop.coverages.push(dataset.to_string());
    }

    let global_index = update_global_stop_index(&mut rubber, stops.iter(), dataset).unwrap();

    info!("Importing {} stops into Mimir", stops.len());
    let nb_stops = rubber.index(dataset, stops.iter()).unwrap();
    info!("Nb of indexed stops: {}", nb_stops);

    publish_global_index(&mut rubber, &global_index).unwrap();
}

fn attach_stop(stop: &mut mimir::Stop, admins: Vec<Rc<mimir::Admin>>) {
    stop.administrative_regions = admins;
    stop.label = format_label(&stop.administrative_regions, &stop.name);
    stop.zip_codes = get_zip_codes_from_admins(&stop.administrative_regions);
}

/// Attach the stops to administrative regions
///
/// The admins are loaded from Elasticsearch and stored in a quadtree
/// We attach a stop with all the admins that have a boundary containing
/// the coordinate of the stop
fn attach_stops_to_admins<'a, It: Iterator<Item = &'a mut mimir::Stop>>(
    stops: It,
    rubber: &mut Rubber,
) {
    let admins = rubber.get_all_admins().unwrap_or_else(|_| {
        info!("Administratives regions not found in elasticsearch db");
        vec![]
    });

    info!("{} administrative regions loaded from mimir", admins.len());

    let admins_geofinder = admins.into_iter().collect::<AdminGeoFinder>();

    let mut nb_unmatched = 0u32;
    let mut nb_matched = 0u32;
    for mut stop in stops {
        let admins = admins_geofinder.get(&stop.coord);

        if admins.is_empty() {
            nb_unmatched += 1;
        } else {
            nb_matched += 1;
        }

        attach_stop(&mut stop, admins);
    }

    info!(
        "there are {}/{} stops without any admin",
        nb_unmatched,
        nb_matched + nb_unmatched
    );
}

fn merge_codes(es_stop: &mut mimir::Stop, codes: Vec<mimir::Code>) {
	let filtered_codes;
	{
		let is_not_existent = |code: &mimir::Code| -> bool {
			for es_code in &es_stop.codes {
				if code.name == es_code.name && code.value == es_code.value {
					return true
				}
			}
			return true
		};
		filtered_codes = codes.into_iter().filter(is_not_existent).collect::<Vec<mimir::Code>>();
	}
	es_stop.codes.extend(filtered_codes.into_iter());	
}

fn merge_physical_modes(es_stop: &mut mimir::Stop, modes: Vec<mimir::PhysicalMode>) {
	let filtered_modes;
	{
		let is_not_existent = |code: &mimir::PhysicalMode| -> bool {
			for es_code in &es_stop.physical_modes {
				if code.id == es_code.id && code.name == es_code.name {
					return true
				}
			}
			return true
		};
		filtered_modes = modes.into_iter().filter(is_not_existent).collect::<Vec<mimir::PhysicalMode>>();
	}
	es_stop.physical_modes.extend(filtered_modes.into_iter());	
}

/// merge the stops from all the different indexes
/// for the moment the merge is very simple and uses only the ID
/// (and we take the data from the first stop inserted)
fn merge_stops<It: IntoIterator<Item = mimir::Stop>>(
    stops: It,
) -> Box<Iterator<Item = mimir::Stop>> {
    let mut stops_by_id = HashMap::<String, mimir::Stop>::new();
    for mut stop in stops.into_iter() {
        let cov = replace(&mut stop.coverages, vec![]);
        let mut codes = replace(&mut stop.codes, vec![]);
        let mut physical_modes = replace(&mut stop.physical_modes, vec![]);
        
        let mut stop_in_map = stops_by_id
            .entry(stop.id.clone())
            .or_insert(stop);
		
		merge_codes(stop_in_map, codes);
		merge_physical_modes(stop_in_map, physical_modes);
		
        stop_in_map.coverages
            .extend(cov.into_iter());
    }
    Box::new(stops_by_id.into_iter().map(|(_, v)| v))
}

fn get_all_stops(rubber: &mut Rubber, index: String) -> Result<Vec<mimir::Stop>, String> {
    rubber
        .get_all_objects_from_index(&index)
        .map_err(|e| e.to_string())
}

fn update_global_stop_index<'a, It: Iterator<Item = &'a mimir::Stop>>(
    rubber: &mut Rubber,
    stops: It,
    dataset: &str,
) -> Result<String, String> {
    let dataset_index = mimir::rubber::get_main_type_and_dataset_index::<mimir::Stop>(dataset);
    let stops_indexes = rubber
        .get_all_aliased_index(&mimir::rubber::get_main_type_index::<mimir::Stop>())?
        .into_iter()
        .filter(|&(_, ref aliases)| !aliases.contains(&dataset_index))
        .map(|(index, _)| index);

    let all_other_es_stops: Vec<_> = stops_indexes
        .map(|index| get_all_stops(rubber, index).unwrap())
        .flat_map(|stops| stops.into_iter())
        .collect();

    let all_es_stops = all_other_es_stops
        .into_iter()
        .chain(stops.into_iter().cloned());

    let all_merged_stops = merge_stops(all_es_stops);
    let es_index_name = mimir::rubber::get_date_index_name(GLOBAL_STOP_INDEX_NAME);

    rubber.create_index(&es_index_name)?;
    let typed_index = TypedIndex::new(es_index_name.clone());

    let nb_stops_added = rubber
        .bulk_index(&typed_index, all_merged_stops)
        .map_err(|e| e.to_string())?;
    info!("{} stops added in the global index", nb_stops_added);
    // create global index
    // fill structure for each stop indexes
    Ok(es_index_name)
}

// publish the global stop index
// alias the new index to the global stop alias, and remove the old index
fn publish_global_index(rubber: &mut Rubber, new_global_index: &str) -> Result<(), String> {
    let last_global_indexes: Vec<_> = rubber
        .get_all_aliased_index(GLOBAL_STOP_INDEX_NAME)?
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k != new_global_index)
        .collect();
    rubber.alias(
        GLOBAL_STOP_INDEX_NAME,
        &vec![new_global_index.to_string()],
        &last_global_indexes,
    )?;

    for index in last_global_indexes {
        rubber.delete_index(&index)?;
    }
    Ok(())
}
