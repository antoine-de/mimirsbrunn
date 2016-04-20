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
extern crate docker_wrapper;
extern crate curl;
extern crate serde_json;
#[macro_use]
extern crate log;

use docker_wrapper::*;
use std::process::Command;

#[test]
fn bano2mimir_sample_test() {
    mimirsbrunn::logger_init().unwrap();
    let wrapper = ElasticSearchDockerWrapper::new().unwrap();

    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    let status = Command::new(bano2mimir)
                     .args(&["--input=./tests/sample-bano.csv".into(),
                             format!("--connection-string={}/munin", wrapper.host())])
                     .status()
                     .unwrap();
    assert!(status.success(), "`bano2mimir` failed {}", &status);

    info!("Refreshing ES indexes");
    let res = curl::http::handle()
                  .get(format!("{}/_refresh", wrapper.host()))
                  .exec()
                  .unwrap();
    assert!(res.get_code() == 200, "Error ES refresh: {}", res);

    //    A way to watch if indexes are built might be curl http://localhost:9200/_stats
    //    then _all/total/segments/index_writer_memory_in_bytes( or version_map_memory_in_bytes)
    // 	  should be == 0 if indexes are ok (no refresh needed)

    let res = curl::http::handle()
                  .get(format!("{}/munin/_search?q=20", wrapper.host()))
                  .exec()
                  .unwrap();
    assert!(res.get_code() == 200, "Error ES search: {}", res);
    let body = std::str::from_utf8(res.get_body()).unwrap();
    debug!("_search?q=20 :\n{}", body);
    let value: serde_json::value::Value = serde_json::from_str(body).unwrap();
    let nb_hits = value.lookup("hits.total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(nb_hits, 1);
}

#[test]
#[should_panic]
fn ko_test() {
    assert!(false);
}
