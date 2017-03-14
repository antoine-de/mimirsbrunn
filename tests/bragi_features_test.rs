extern crate bragi;
extern crate iron_test;
extern crate serde_json;
use super::BragiHandler;
use super::get_values;
use super::get_value;
use super::get_types;
use super::count_types;


pub fn bragi_features_test(es_wrapper: ::ElasticSearchWrapper) {
    let bragi = BragiHandler::new(format!("{}/munin", es_wrapper.host()));

    // *********************************
    // We load the OSM dataset and three-cities bano dataset
    // the current dataset are thus (load order matters):
    // - osm_fixture.osm.pbf (including ways)
    // - bano-three_cities
    // *********************************
    let osm2mimir = concat!(env!("OUT_DIR"), "/../../../osm2mimir");
    ::launch_and_assert(osm2mimir,
                        vec!["--input=./tests/fixtures/osm_fixture.osm.pbf".into(),
                             "--import-way".into(),
                             "--level=8".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    ::launch_and_assert(bano2mimir,
                        vec!["--input=./tests/fixtures/bano-three_cities.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);

    let stops2mimir = concat!(env!("OUT_DIR"), "/../../../stops2mimir");
    ::launch_and_assert(stops2mimir,
                        vec!["--input=./tests/fixtures/stops.txt".into(),
                             "--dataset=dataset1".into(),
                             format!("--connection-string={}", es_wrapper.host())],
                        &es_wrapper);
    
    addr_by_id_test(&bragi);
    admin_by_id_test(&bragi);
    street_by_id_test(&bragi);
    stop_by_id_test(&bragi);
}

fn admin_by_id_test(bragi: &BragiHandler) {
    let all_20 = bragi.get("/features/admin:fr:77288");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);
    let count = count_types(&types, "city");
    assert_eq!(count, 1);

    assert_eq!(get_values(&all_20, "id"),
               vec!["admin:fr:77288"]);
}

fn street_by_id_test(bragi: &BragiHandler) {
    let all_20 = bragi.get("/features/161162362");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);
    
    let count = count_types(&types, "street");
    assert_eq!(count, 1);

    assert_eq!(get_values(&all_20, "id"),
               vec!["161162362"]);
}

fn addr_by_id_test(bragi: &BragiHandler) {
    let all_20 = bragi.get("/features/addr:2.68385;48.50539");
    assert_eq!(all_20.len(), 1);
    let types = get_types(&all_20);
    let count = count_types(&types, "house");
    assert_eq!(count, 1);
    assert_eq!(get_values(&all_20, "id"),
               vec!["addr:2.68385;48.50539"]);
}

fn stop_by_id_test(bragi: &BragiHandler) {
    // search with id
    let response = bragi.get("/features/stop_area:SA:second_station?pt_dataset=dataset1");
    assert_eq!(response.len(), 1);
    let stop = response.first().unwrap();
    assert_eq!(get_value(stop, "id"),
               "stop_area:SA:second_station");

}
