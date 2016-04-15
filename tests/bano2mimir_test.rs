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

extern crate mimirsbrunn;
extern crate curl;
extern crate serde_json;
#[macro_use]
extern crate log;

mod elastic_search_docker_wrapper;

use std::process::Command;

#[test]
fn bano2mimir_sample_test() {
    mimirsbrunn::logger_init().unwrap();
    let wrapper = elastic_search_docker_wrapper::new_es_docker_wrapper();

    info!("Launching ES docker");
    let status = Command::new("docker")
                     .args(&["run",
                             "--publish=9200:9200",
                             "--publish=9300:9300",
                             "-d",
                             "--name=mimirsbrunn_tests",
                             "elasticsearch"])
                     .status()
                     .unwrap();
    assert!(status.success(), "`docker run` failed {}", &status);

    info!("Waiting for ES to be up and running...");
    let step_duration = 100;
    let mut is_docker_up = false;
    for nb_try in 0..200 {
        let res = curl::http::handle()
                      .get(wrapper.connection_string)
                      .exec()
                      .map(|res| res.get_code() == 200)
                      .unwrap_or(false);
        if res {
            info!("ES up and running after {} ms.", nb_try * step_duration);
            is_docker_up = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(step_duration));
    }
    assert!(is_docker_up, "ES is down");

    info!("Launching bano2mimir");
    let status = Command::new("./target/debug/bano2mimir")
                     .arg("--input=./tests/sample-bano.csv")
                     .status()
                     .unwrap();
    assert!(status.success(), "`bano2mimir` failed {}", &status);

    info!("Refreshing ES indexes");
    let res = curl::http::handle()
                  .get(format!("{}/_refresh", wrapper.connection_string))
                  .exec()
                  .unwrap();
    assert!(res.get_code() == 200, "Error ES refresh: {}", res);

    //    A way to watch if indexes are built might be curl http://localhost:9200/_stats
    //    then _all/total/segments/index_writer_memory_in_bytes( or version_map_memory_in_bytes)
    // 	  should be == 0 if indexes are ok (no refresh needed)

    let res = curl::http::handle()
                  .get(format!("{}/munin/_search?q=20", wrapper.connection_string))
                  .exec()
                  .unwrap();
    assert!(res.get_code() == 200, "Error ES search: {}", res);
    let body = std::str::from_utf8(res.get_body()).unwrap();
    debug!("_search?q=20 :\n{:?}", body);
    let value: serde_json::value::Value = serde_json::from_str(body).unwrap();
    let nb_hits = value.lookup("hits.total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(nb_hits, 1);

    info!("Stopping docker");
    let status = Command::new("docker").args(&["stop", "mimirsbrunn_tests"]).status().unwrap();
    assert!(status.success());

    info!("Deleting container");
    let status = Command::new("docker").args(&["rm", "mimirsbrunn_tests"]).status().unwrap();
    assert!(status.success());
}

#[test]
#[should_panic]
fn ko_test() {
    assert!(false);
}
