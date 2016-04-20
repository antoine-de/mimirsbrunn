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

mod bano2mimir_test;

// TODO: should probably be a struct (maybe implementing Drop)
fn init_es(wrapper: &ElasticSearchDockerWrapper) -> &'static str {
    let mut rubber = mimirsbrunn::rubber::Rubber::new(&format!("{}/_all", wrapper.host()));
    rubber.delete_index().unwrap();
    wrapper.host()
}

#[test]
fn all_tests() {
    mimirsbrunn::logger_init().unwrap();
    let wrapper = ElasticSearchDockerWrapper::new().unwrap();

    bano2mimir_test::bano2mimir_sample_test(init_es(&wrapper));
}
