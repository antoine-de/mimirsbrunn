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

extern crate mimir;
extern crate docker_wrapper;
extern crate curl;
extern crate serde_json;
#[macro_use]
extern crate log;

use docker_wrapper::*;

mod bano2mimir_test;

pub struct ElasticSearchWrapper<'a> {
    docker_wrapper: &'a DockerWrapper,
}

impl<'a> ElasticSearchWrapper<'a> {
    pub fn host(&self) -> String {
        self.docker_wrapper.host()
    }

    pub fn init(&self) {
        let mut rubber = mimir::rubber::Rubber::new(&format!("{}/_all",
                                                                   self.docker_wrapper.host()));
        rubber.delete_index().unwrap();
    }

    //    A way to watch if indexes are built might be curl http://localhost:9200/_stats
    //    then _all/total/segments/index_writer_memory_in_bytes( or version_map_memory_in_bytes)
    // 	  should be == 0 if indexes are ok (no refresh needed)
    pub fn refresh(&self) {
        info!("Refreshing ES indexes");
        let res = ::curl::http::handle()
                      .get(format!("{}/_refresh", self.host()))
                      .exec()
                      .unwrap();
        assert!(res.get_code() == 200, "Error ES refresh: {}", res);
    }

    pub fn new(docker_wrapper: &DockerWrapper) -> ElasticSearchWrapper {
        let es_wrapper = ElasticSearchWrapper { docker_wrapper: docker_wrapper };
        es_wrapper.init();
        es_wrapper
    }
}

/// Main test method (regroups all tests)
/// All tests are done sequentially,
/// and use the same docker in order to avoid multiple inits
/// (ES cleanup is handled by es_wrapper)
#[test]
fn all_tests() {
    mimir::logger_init().unwrap();
    let docker_wrapper = DockerWrapper::new().unwrap();

    // we call all tests here
    bano2mimir_test::bano2mimir_sample_test(ElasticSearchWrapper::new(&docker_wrapper));
}
