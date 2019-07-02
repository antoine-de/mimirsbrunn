// Copyright © 2019, Canal TP and/or its affiliates. All rights reserved.
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

use super::get_first_index_aliases;
use reqwest;
use std::path::Path;

/// Inserts POI into Elasticsearch and check results.
/// Prior to POI insertion, we need to insert contextual data:
/// - cosmogony (so that we get the admins),
/// - bano (so that we can attach the POI to an address)
pub fn poi2mimir_sample_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let out_dir = Path::new(env!("OUT_DIR"));

    let cosmogony2mimir = out_dir
        .join("../../../cosmogony2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &cosmogony2mimir,
        &[
            "--input=./tests/fixtures/cosmogony.json".into(),
            "--lang=fr".into(),
            "--lang=es".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let bano2mimir = out_dir.join("../../../bano2mimir").display().to_string();
    crate::launch_and_assert(
        &bano2mimir,
        &[
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let poi2mimir = out_dir.join("../../../poi2mimir").display().to_string();
    crate::launch_and_assert(
        &poi2mimir,
        &[
            "--input=./tests/fixtures/poi/test.poi".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    // Make sure import went smoothly by checking the number of indices.

    // after an import, we should have 1 index, and some aliases to this index
    let mut res = reqwest::get(&format!("{host}/_aliases", host = es_wrapper.host())).unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::value::Value = res.json().unwrap();
    let raw_indexes = json.as_object().unwrap();
    let first_indexes: Vec<String> = raw_indexes.keys().cloned().collect();

    assert_eq!(first_indexes.len(), 3); // 3 indexes
                                        // our index should be aliased by the master_index + an alias over the document type + dataset
    let aliases = get_first_index_aliases(raw_indexes);

    // for the moment 'munin' is hard coded, but hopefully that will change
    assert_eq!(
        aliases,
        vec!["munin", "munin_addr", "munin_addr_fr", "munin_geo_data"]
    );

    // Making sure that there are as many POI in ES as there are the input file.
    assert_eq!(es_wrapper.count("_type:poi"), 4);

    // We test that the POI that was close to an address has been 'attached' to that address,
    // and inherited its admin and address: (Note that the data have been manipulated to fit)
    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Agence Four", |_| true)
        .collect();
    assert!(res.len() != 0);
    assert!(res[0].is_poi());

    assert!(res[0]
        .admins()
        .iter()
        .filter(|adm| adm.is_city())
        .any(|adm| adm.name == "Livry-sur-Seine"));

    // We test the opposite of the previous case: a POI that is far from any address
    let res: Vec<_> = es_wrapper
        .search_and_filter("label:Station Bellecour", |_| true)
        .collect();
    assert!(res.len() != 0);
    assert!(res[0].is_poi());

    assert!(res[0].admins().is_empty());
    assert!(res[0].address().is_none());
}
