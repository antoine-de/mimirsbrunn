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

use std::process::Command;
use hyper;
use hyper::client::Client;
use hyper::client::response::Response;
//use rs_es;
//use rs_es::EsResponse;
use serde_json;
use serde_json::value::Value;

trait ToJson {
    fn to_json(self) -> Value;
}
impl ToJson for Response {
    fn to_json(self) -> Value {
        match serde_json::from_reader(self) {
            Ok(v) => v,
            Err(e) => { assert!(false, "could not get json value from response: {:?}", e); panic!("should not be possible") }
        }
    }
}

/// Simple call to a BANO load into ES base
/// Checks that we are able to find one object (a specific address)
pub fn bano2mimir_sample_test(es_wrapper: ::ElasticSearchWrapper) {
    let bano2mimir = concat!(env!("OUT_DIR"), "/../../../bano2mimir");
    info!("Launching {}", bano2mimir);
    let status = Command::new(bano2mimir)
                     .args(&["--input=./tests/sample-bano.csv".into(),
                             format!("--connection-string={}", es_wrapper.host())])
                     .status()
                     .unwrap();
    assert!(status.success(), "`bano2mimir` failed {}", &status);

    es_wrapper.refresh();
    let master_index = "munin"; // for the moment it's hard coded, but hopefully that will change

    let client = Client::new();
    let res = client.get(&format!("{host}/{index}/_search?q=20",
                  host=es_wrapper.host(),
                  index=master_index)).send().unwrap();
    assert!(res.status == hyper::Ok);
    let value = res.to_json();

    let nb_hits = value.lookup("hits.total").and_then(|v| v.as_u64()).unwrap_or(0);
    assert_eq!(nb_hits, 1);

    // after an import, we should have 1 index, and some aliases to this index
    let res = client.get(&format!("{host}/_aliases", host=es_wrapper.host()))
                  .send()
                  .unwrap();
    assert!(res.status == hyper::Ok);
}
