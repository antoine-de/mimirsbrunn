// Copyright © 2017, Canal TP and/or its affiliates. All rights reserved.
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

use super::count_types;
use super::filter_by;
use super::get_poi_type_ids;
use super::get_types;
use super::get_value;
use super::get_values;
use super::BragiHandler;
use mimir::{MimirObject, Poi};
use serde_json::json;
use std::path::Path;

pub fn bragi_poi_test(es_wrapper: crate::ElasticSearchWrapper<'_>) {
    let mut bragi = BragiHandler::new(es_wrapper.host());

    // ******************************************
    // We load three-cities bano dataset and then the OSM dataset (with the POIs)
    // the current dataset are thus (load order matters):
    // - bano-three_cities
    // - osm_fixture.osm.pbf (including ways and pois)
    // ******************************************
    let bano2mimir = Path::new(env!("OUT_DIR"))
        .join("../../../bano2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &bano2mimir,
        &[
            "--input=./tests/fixtures/bano-three_cities.csv".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    let osm2mimir = Path::new(env!("OUT_DIR"))
        .join("../../../osm2mimir")
        .display()
        .to_string();
    crate::launch_and_assert(
        &osm2mimir,
        &[
            "--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
            "--import-admin".into(),
            "--import-way".into(),
            "--import-poi".into(),
            "--level=8".into(),
            format!("--connection-string={}", es_wrapper.host()),
        ],
        &es_wrapper,
    );

    poi_admin_address_test(&mut bragi);
    poi_admin_test(&mut bragi);
    poi_zip_code_test(&mut bragi);
    poi_from_osm_test(&mut bragi);
    poi_misspelt_one_word_admin_test(&mut bragi);
    poi_from_osm_with_address_addr_test(&mut bragi);
    poi_filter_poi_type_test(&mut bragi);
    poi_filter_error_message_test(&mut bragi);
}

fn poi_admin_address_test(bragi: &mut BragiHandler) {
    let geocodings = bragi.get("/autocomplete?q=Le-Mée-sur-Seine Courtilleraies");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);

    // the first element returned should be the poi 'Le-Mée-sur-Seine Courtilleraies'
    let poi = geocodings.first().unwrap();
    let properties = poi
        .get("properties")
        .and_then(|json| json.as_array())
        .unwrap();
    let keys = [
        "addr:postcode",
        "amenity",
        "atm",
        "name",
        "operator",
        "phone",
        "ref:FR:LaPoste",
        "source",
        "wheelchair",
    ];
    for p in properties {
        let key = p["key"].as_str().unwrap();
        assert!(keys.contains(&key));
    }
    assert_eq!(properties.len(), keys.len());
    assert_eq!(get_value(poi, "type"), Poi::doc_type());
    assert_eq!(get_value(poi, "label"), "Le-Mée-sur-Seine Courtilleraies");
    assert_eq!(get_value(poi, "postcode"), "77350");
}

fn poi_admin_test(bragi: &mut BragiHandler) {
    // with this search we should be able to find a poi called Melun Rp
    let geocodings = bragi.get("/autocomplete?q=Melun Rp");
    let types = get_types(&geocodings);
    let count = count_types(&types, Poi::doc_type());
    assert!(count >= 1);

    assert!(get_values(&geocodings, "label").contains(&"Melun Rp (Melun)",));

    // when we search for just 'Melun', we should find some places in melun
    let geocodings = bragi.get("/autocomplete?q=Melun");
    for postcodes in get_values(&geocodings, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    // we should also be able to find the city of melun which will carry more postcodes
    let cities = filter_by(&geocodings, "zone_type", "city");
    assert_eq!(cities.len(), 1);
    let melun = &cities.first().unwrap();
    assert_eq!(get_value(melun, "name"), "Melun");
    assert_eq!(get_value(melun, "postcode"), "77000;77003;77008;CP77001");
    assert_eq!(get_value(melun, "label"), "Melun (77000-CP77001)");
}

fn poi_zip_code_test(bragi: &mut BragiHandler) {
    // search by zip code
    let geocodings = bragi.get("/autocomplete?q=77000&limit=15");
    let types = get_types(&geocodings);
    let zone_types = get_values(&geocodings, "zone_type");
    assert_eq!(count_types(&types, Poi::doc_type()), 2);
    assert_eq!(count_types(&zone_types, "city"), 3);
    assert_eq!(count_types(&types, "street"), 8);

    // search by zip code and limit is string type
    let geocodings = bragi.get_unchecked_json("/autocomplete?q=77000&limit=ABCD");
    assert_eq!(
        geocodings,
        (
            actix_web::http::StatusCode::BAD_REQUEST,
            json!({
                "short": "validation error",
                "long": "invalid argument: failed with reason: invalid digit found in string",
            })
        )
    );

    // search by zip code and limit < 0
    let geocodings = bragi.get_unchecked_json("/autocomplete?q=77000&limit=-1");
    assert_eq!(
        geocodings,
        (
            actix_web::http::StatusCode::BAD_REQUEST,
            json!({
                "short": "validation error",
                "long": "invalid argument: failed with reason: invalid digit found in string",
            })
        )
    );

    // search by zip code and limit and offset
    let all_20 = bragi.get("/autocomplete?q=77000&limit=10&offset=0");
    assert_eq!(all_20.len(), 10);

    let all_20 = bragi.get("/autocomplete?q=77000&limit=10&offset=10");
    assert_eq!(all_20.len(), 3);
}

fn poi_from_osm_test(bragi: &mut BragiHandler) {
    // search poi: Poi as a relation in osm data
    let geocodings = bragi.get("/autocomplete?q=Parking (Le Coudray-Montceaux)");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);
    let first_poi = geocodings
        .iter()
        .find(|e| get_value(e, "type") == Poi::doc_type())
        .unwrap();
    assert_eq!(get_value(first_poi, "citycode"), "91179");
    assert_eq!(get_poi_type_ids(first_poi), &["poi_type:amenity:parking"]);

    // search poi: Poi as a way in osm data
    let geocodings = bragi.get("/autocomplete?q=77000 Hôtel de Ville (Melun)");
    let types = get_types(&geocodings);
    assert!(count_types(&types, Poi::doc_type()) >= 1);
    let poi = geocodings.first().unwrap();
    assert_eq!(get_value(poi, "type"), Poi::doc_type());
    assert_eq!(get_value(poi, "id"), "poi:osm:way:112361498");
    assert_eq!(get_value(poi, "label"), "Hôtel de Ville (Melun)");
    assert_eq!(get_poi_type_ids(poi), &["poi_type:amenity:townhall"]);

    // we search for POIs with a type but an empty name, we should have set the name with the type.
    // for exemple there are parkings without name (but with the tag "anemity" = "Parking"),
    // we should be able to query them
    let geocodings = bragi.get("/autocomplete?q=Parking");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 8);

    // we search for a POI (id = 2561223) with a label but an empty coord
    // (barycenter not computed so far), it should be filtered.
    let geocodings = bragi.get("/autocomplete?q=ENSE3 site Ampère");
    // we can find other results (due to the fuzzy search, but we can't find the 'site Ampère')
    assert!(!get_values(&geocodings, "label").contains(&"ENSE3 site Ampère",));
}

fn poi_misspelt_one_word_admin_test(bragi: &mut BragiHandler) {
    // with this search we should be able to find a poi called "Melun"
    let geocodings = bragi.get("/autocomplete?q=Melun");
    let types = get_types(&geocodings);
    let count = count_types(&types, Poi::doc_type());
    assert!(count >= 1);
    assert!(get_values(&geocodings, "label").contains(&"Melun Rp (Melun)",));

    // when we search for 'Meluuun', we should find some places in melun
    let geocodings = bragi.get("/autocomplete?q=Meluuun");
    for postcodes in get_values(&geocodings, "postcode") {
        assert!(postcodes.split(';').any(|p| p == "77000"));
    }
    // we should also be able to find the city of melun which will carry more postcodes
    let cities = filter_by(&geocodings, "zone_type", "city");
    assert_eq!(cities.len(), 1);
    let melun = &cities.first().unwrap();
    assert_eq!(get_value(melun, "name"), "Melun");
}

fn poi_from_osm_with_address_addr_test(bragi: &mut BragiHandler) {
    // search poi: Poi as a way in osm data
    let geocodings = bragi.get("/autocomplete?q=77000 Hôtel de Ville (Melun)");
    let types = get_types(&geocodings);
    assert!(count_types(&types, Poi::doc_type()) >= 1);
    let poi = geocodings.first().unwrap();
    assert_eq!(get_value(poi, "type"), Poi::doc_type());
    assert_eq!(get_value(poi, "id"), "poi:osm:way:112361498");

    assert_eq!(
        poi.get("address").and_then(|a| a.pointer("/housenumber")),
        Some(&json!("2"))
    );
    assert_eq!(
        poi.get("address").and_then(|a| a.pointer("/id")),
        Some(&json!("addr:2.65801;48.53685:2"))
    );
    assert_eq!(
        poi.get("address").and_then(|a| a.pointer("/label")),
        Some(&json!("2 Rue de la Reine Blanche (Melun)"))
    );
}

// test 'labels' and 'names' fields work with i18n queries
pub fn test_i18n_poi(mut es: crate::ElasticSearchWrapper<'_>) {
    // we define a simple test italian poi (the 'Colosseo'
    // with 2 langs for labels and names fields ('fr' and 'es')
    let coord = mimir::Coord(geo::Coordinate { x: 0.0, y: 0.0 });
    let colosseo = mimir::Poi {
        id: "".to_string(),
        label: "Colosseo (Roma)".to_string(),
        name: "Colosseo".to_string(),
        coord: coord.clone(),
        approx_coord: Some(coord.into()),
        administrative_regions: vec![],
        weight: 0.0,
        zip_codes: vec![],
        poi_type: mimir::PoiType {
            id: "".to_string(),
            name: "".to_string(),
        },
        properties: vec![],
        address: None,
        names: mimir::I18nProperties(vec![
            mimir::Property {
                key: "fr".to_string(),
                value: "Colisée".to_string(),
            },
            mimir::Property {
                key: "es".to_string(),
                value: "Coliseo".to_string(),
            },
        ]),
        labels: mimir::I18nProperties(vec![
            mimir::Property {
                key: "fr".to_string(),
                value: "Colisée (Rome)".to_string(),
            },
            mimir::Property {
                key: "es".to_string(),
                value: "Coliseo (Roma)".to_string(),
            },
        ]),
        ..Default::default()
    };

    let index_settings = mimir::rubber::IndexSettings {
        nb_shards: 2,
        nb_replicas: 1,
    };
    // we index the poi above
    let _result = es
        .rubber
        .index("munin_poi", &index_settings, std::iter::once(colosseo));

    es.refresh();

    let mut bragi = BragiHandler::new(format!("{}/munin", es.host()));

    // We look for the Colisée in spanish
    let poi = bragi.get("/autocomplete?q=Coliseo&lang=es");
    let result = poi.first().unwrap();
    assert_eq!(result["name"], "Coliseo");
    assert_eq!(result["label"], "Coliseo (Roma)");

    // We look for the Colisée in french
    let poi = bragi.get("/autocomplete?q=Colisée&lang=fr");
    let result = poi.first().unwrap();
    assert_eq!(result["name"], "Colisée");
    assert_eq!(result["label"], "Colisée (Rome)");

    // We look for the Colisée in italian: since it has not been
    // indexed in russian we expect the default name and label (ie the
    // local ones: italian).
    let poi = bragi.get("/autocomplete?q=Colosseo&lang=it");
    let result = poi.first().unwrap();
    assert_eq!(result["name"], "Colosseo");
    assert_eq!(result["label"], "Colosseo (Roma)");
}

fn poi_filter_poi_type_test(bragi: &mut BragiHandler) {
    let geocodings =
        bragi.get("/autocomplete?q=77000&type[]=poi&poi_type[]=poi_type:amenity:post_office");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);

    let geocodings =
        bragi.get("/autocomplete?q=77000&type[]=poi&poi_type[]=poi_type:amenity:townhall");
    let types = get_types(&geocodings);
    assert_eq!(count_types(&types, Poi::doc_type()), 1);
}

fn poi_filter_error_message_test(bragi: &mut BragiHandler) {
    let geocodings = bragi.get_unchecked_json(
        "/autocomplete?q=77000&type[]=zone&poi_type[]=poi_type:amenity:post_office",
    );
    assert_eq!(
        geocodings,
        (
            actix_web::http::StatusCode::BAD_REQUEST,
            json!({
                "short": "validation error",
                "long": "Invalid parameter: poi_type[] parameter requires to have 'type[]=poi'",
            })
        )
    );
}
