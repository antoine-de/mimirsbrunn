// Copyright © 2018, Canal TP and/or its affiliates. All rights reserved.
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

use failure::format_err;
use futures::stream::StreamExt;

use crate::utils::DEFAULT_NB_THREADS;
use common::config::load_es_config_for;
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::{
    adapters::secondary::elasticsearch::{self, ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ},
    domain::ports::secondary::remote::Remote,
};
use mimirsbrunn::addr_reader::{import_addresses_from_files, import_addresses_from_reads};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::{labels, utils};
use places::addr::Addr;
use serde::{Deserialize, Serialize};
use slog_scope::{info, warn};
use std::io::stdin;
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct OpenAddress {
    pub id: String,
    pub street: String,
    pub postcode: String,
    pub district: String,
    pub region: String,
    pub city: String,
    pub number: String,
    pub unit: String,
    pub lat: f64,
    pub lon: f64,
}

impl OpenAddress {
    pub fn into_addr(
        self,
        admins_geofinder: &AdminGeoFinder,
        id_precision: usize,
    ) -> Result<Addr, mimirsbrunn::Error> {
        let street_id = format!("street:{}", self.id); // TODO check if thats ok
        let admins = admins_geofinder.get(&geo::Coordinate {
            x: self.lon,
            y: self.lat,
        });
        let country_codes = utils::find_country_codes(admins.iter().map(|a| a.deref()));

        let weight = admins.iter().find(|a| a.is_city()).map_or(0., |a| a.weight);
        // Note: for openaddress, we don't trust the admin hierarchy much (compared to bano)
        // so we use for the label the admins that we find in the DB
        let street_label = labels::format_street_label(
            &self.street,
            admins.iter().map(|a| a.deref()),
            &country_codes,
        );
        let (addr_name, addr_label) = labels::format_addr_name_and_label(
            &self.number,
            &self.street,
            admins.iter().map(|a| a.deref()),
            &country_codes,
        );

        let zip_codes: Vec<_> = self.postcode.split(';').map(str::to_string).collect();
        let coord = places::coord::Coord::new(self.lon, self.lat);
        let street = places::street::Street {
            id: street_id,
            name: self.street,
            label: street_label,
            administrative_regions: admins,
            weight,
            zip_codes: zip_codes.clone(),
            coord,
            approx_coord: None,
            distance: None,
            country_codes: country_codes.clone(),
            context: None,
        };

        let id_suffix = format!(
            ":{}",
            self.number
                .replace(" ", "")
                .replace("\t", "")
                .replace("\r", "")
                .replace("\n", "")
                .replace("/", "-")
                .replace(".", "-")
                .replace(":", "-")
                .replace(";", "-")
        );

        let id = {
            if id_precision > 0 {
                format!(
                    "addr:{:.precision$};{:.precision$}{}",
                    self.lon,
                    self.lat,
                    id_suffix,
                    precision = id_precision
                )
            } else {
                format!("addr:{};{}{}", self.lon, self.lat, id_suffix)
            }
        };

        Ok(Addr {
            id,
            name: addr_name,
            label: addr_label,
            house_number: self.number,
            street,
            coord,
            approx_coord: Some(coord.into()),
            weight,
            zip_codes,
            distance: None,
            country_codes,
            context: None,
        })
    }
}

#[derive(StructOpt, Debug)]
struct Args {
    /// OpenAddresses files. Can be either a directory or a file.
    /// If this is left empty, addresses are read from standard input.
    #[structopt(short = "i", long, parse(from_os_str))]
    input: Option<PathBuf>,
    /// Float precision for coordinates used to define the `id` field of addresses.
    /// Set to 0 to use exact coordinates.
    #[structopt(short = "p", long, default_value = "6")]
    id_precision: usize,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long, default_value = "http://localhost:9200/munin")]
    connection_string: String,
    /// Number of threads to use
    #[structopt(short = "t", long, default_value = &DEFAULT_NB_THREADS)]
    nb_threads: usize,
    /// Number of threads to use to insert into Elasticsearch. Note that Elasticsearch is not able
    /// to handle values that are too high.
    #[structopt(short = "T", long, default_value = "1")]
    nb_insert_threads: usize,
    #[structopt(parse(from_os_str), long)]
    mappings: Option<PathBuf>,
    #[structopt(parse(from_os_str), long)]
    settings: Option<PathBuf>,
    /// Override value of settings using syntax `key.subkey=val`
    #[structopt(name = "setting", short = "v", long)]
    override_settings: Vec<String>,
}

async fn run(args: Args) -> Result<(), failure::Error> {
    info!("importing open addresses into Mimir");

    let config = load_es_config_for::<Addr>(args.mappings, args.settings, args.override_settings)
        .map_err(|err| format_err!("could not load configuration: {}", err))?;

    let pool = elasticsearch::remote::connection_pool_url(&args.connection_string)
        .await
        .map_err(|err| {
            format_err!(
                "could not create elasticsearch connection pool: {}",
                err.to_string()
            )
        })?;

    let client = pool
        .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
        .await
        .map_err(|err| format_err!("could not connect elasticsearch pool: {}", err.to_string()))?;

    // Fetch and index admins for `into_addr`
    let into_addr = {
        let admins_geofinder = match client.list_documents().await {
            Ok(stream) => {
                stream
                    .map(|admin| admin.expect("could not parse admin"))
                    .collect()
                    .await
            }
            Err(err) => {
                warn!("administratives regions not found in es db. {:?}", err);
                Default::default()
            }
        };
        let id_precision = args.id_precision;
        move |a: OpenAddress| a.into_addr(&admins_geofinder, id_precision)
    };

    if let Some(input_path) = args.input {
        // Import from file(s)
        if input_path.is_dir() {
            let paths = walkdir::WalkDir::new(&input_path);
            let path_iter = paths
                .into_iter()
                .map(|p| p.unwrap().into_path())
                .filter(|p| {
                    let f = p
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.ends_with(".csv") || name.ends_with(".csv.gz"))
                        .unwrap_or(false);
                    if !f {
                        info!("skipping file {} as it is not a csv", p.display());
                    }
                    f
                });

            import_addresses_from_files(client, config, true, args.nb_threads, path_iter, into_addr)
                .await
        } else {
            import_addresses_from_files(
                client,
                config,
                true,
                args.nb_threads,
                std::iter::once(input_path),
                into_addr,
            )
            .await
        }
    } else {
        // Import from stdin
        import_addresses_from_reads(
            client,
            config,
            true,
            args.nb_threads,
            vec![stdin()],
            into_addr,
        )
        .await
    }
}

#[tokio::main]
async fn main() {
    mimirsbrunn::utils::launch_async(Box::new(run)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::document::ContainerDocument;
    use futures::TryStreamExt;
    use mimir2::domain::model::query::Query;
    use mimir2::domain::ports::primary::list_documents::ListDocuments;
    use mimir2::domain::ports::primary::search_documents::SearchDocuments;
    use mimir2::{adapters::secondary::elasticsearch::remote, utils::docker};
    use places::{addr::Addr, Place};

    fn elasticsearch_test_url() -> String {
        std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
    }

    #[tokio::test]
    async fn should_correctly_index_oa_file() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let args = Args {
            input: Some("./tests/fixtures/sample-oa.csv".into()),
            connection_string: elasticsearch_test_url(),
            mappings: Some("./config/addr/mappings.json".into()),
            settings: Some("./config/addr/settings.json".into()),
            id_precision: 5,
            nb_threads: 2,
            nb_insert_threads: 2,
            override_settings: vec![],
        };

        let _res = mimirsbrunn::utils::launch_async_args(run, args).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");

        let search = |query: &str| {
            let client = client.clone();
            let query: String = query.into();
            async move {
                client
                    .search_documents(
                        vec![String::from(Addr::static_doc_type())],
                        Query::QueryString(format!("full_label.prefix:({})", query)),
                    )
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|json| serde_json::from_value::<Place>(json).unwrap())
                    .map(|place| match place {
                        Place::Addr(addr) => addr,
                        _ => panic!("should only have admins"),
                    })
                    .collect::<Vec<Addr>>()
            }
        };

        let addresses: Vec<Addr> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        drop(guard);

        assert_eq!(addresses.len(), 11);

        let results = search("Otto-Braun-Straße 72").await;
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.id, "addr:13.41931;52.52354:72");

        // We look for postcode 11111 which should have been filtered since the street name is empty
        let results = search("11111").await;
        assert_eq!(results.len(), 0);

        // Check that addresses containing multiple postcodes are read correctly
        let results = search("Rue Foncet").await;
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].zip_codes,
            vec!["06000", "06100", "06200", "06300"]
        )
    }
}
