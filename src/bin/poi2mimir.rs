// Copyright © 2023, Hove and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Hove (www.kisio.com).
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

use clap::Parser;
use mimirsbrunn::poi2mimir::{run, Error};
use mimirsbrunn::settings::poi2mimir as settings;

fn main() -> Result<(), Error> {
    let opts = settings::Opts::parse();
    let settings = settings::Settings::new(&opts).map_err(|e| Error::Settings { source: e })?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::launch_with_runtime(
            settings.nb_threads,
            run(opts, settings),
        )
        .map_err(|e| Error::Execution { source: e }),
        settings::Command::Config => {
            println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::tests::{bano, cosmogony, osm};
    use futures::TryStreamExt;
    use mimir::{
        adapters::secondary::elasticsearch::{remote, ElasticsearchStorageConfig},
        domain::ports::{primary::list_documents::ListDocuments, secondary::remote::Remote},
        utils::docker,
    };
    use mimirsbrunn::settings::poi2mimir as settings;
    use places::poi::Poi;
    use serial_test::serial;
    use test_log::test;

    #[test(tokio::test)]
    #[serial]
    async fn should_correctly_index_poi_file() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        // We need to prep the test by inserting admins, addresses, and streets.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
            .await
            .expect("Elasticsearch Connection Established");

        cosmogony::index_admins(&client, "corse", "corse", true, true)
            .await
            .unwrap();

        osm::index_streets(&client, "limousin", "limousin", true)
            .await
            .unwrap();

        bano::index_addresses(&client, "limousin", "limousin", true)
            .await
            .unwrap();

        // And here is the indexing of Pois...
        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "corse.poi"]
                .iter()
                .collect(),
            cmd: settings::Command::Run,
        };

        let settings = settings::Settings::new(&opts).unwrap();

        mimirsbrunn::utils::launch::launch_async(move || run(opts, settings))
            .await
            .unwrap();

        // Now we query the index we just created. Since it's a small poi file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
            .await
            .expect("Elasticsearch Connection Established");

        let pois: Vec<Poi> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(pois.len(), 1);
    }
}
