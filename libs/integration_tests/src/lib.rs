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

extern crate curl;
extern crate retry;
#[macro_use]
extern crate log;

use std::process::Command;
use std::error::Error;

pub struct ElasticSearchDockerWrapper {
    host: &'static str,
}

impl ElasticSearchDockerWrapper {
    pub fn host(&self) -> &'static str {
        self.host
    }

    fn setup(host: &'static str) -> Result<(), Box<Error>> {
        info!("Launching ES docker");
        let status = try!(Command::new("docker")
                              .args(&["run",
                                      "--publish=9242:9200",
                                      "-d",
                                      "--name=mimirsbrunn_tests",
                                      "elasticsearch"])
                              .status());
        if !status.success() {
            return Err(format!("`docker run` failed {}", &status).into());
        }

        info!("Waiting for ES in docker to be up and running...");
        match retry::retry(200,
                           100,
                           || curl::http::handle().get(host).exec(),
                           |response| {
                               response.as_ref()
                                       .map(|res| res.get_code() == 200)
                                       .unwrap_or(false)
                           }) {
            Ok(_) => Ok(()),
            Err(_) => Err("ES is down".into()),
        }
    }

    pub fn new() -> Result<ElasticSearchDockerWrapper, Box<Error>> {
        let wrapper = ElasticSearchDockerWrapper { host: "localhost:9242" };
        try!(ElasticSearchDockerWrapper::setup(wrapper.host));
        Ok(wrapper)
    }
}

fn docker_command(args: &[&'static str]) {
    info!("Running docker {:?}", args);
    let status = Command::new("docker").args(args).status();
    match status {
        Ok(s) => {
            if !s.success() {
                warn!("`docker {a:?}` failed {s}", a = args, s = s)
            }
        }
        Err(e) => warn!("command `docker {a:?}` failed {e}", a = args, e = e),
    }
}

impl Drop for ElasticSearchDockerWrapper {
    fn drop(&mut self) {
        docker_command(&["stop", "mimirsbrunn_tests"]);
        docker_command(&["rm", "mimirsbrunn_tests"]);
    }
}
