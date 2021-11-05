use config::Config;
use futures::stream::StreamExt;
use snafu::{ResultExt, Snafu};
use structopt::StructOpt;
use tracing::{instrument, warn};

use mimir::adapters::secondary::elasticsearch::{self, ElasticsearchStorage};
use mimir::domain::model::index::IndexVisibility;
use mimir::domain::ports::primary::{generate_index::GenerateIndex, list_documents::ListDocuments};
use mimir::domain::ports::secondary::remote::Remote;
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use mimirsbrunn::osm_reader::street::streets;
use mimirsbrunn::settings::osm2mimir as settings;
use places::poi::Poi;
use places::street::Street;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("OSM PBF Reader Error: {}", source))]
    OsmPbfReader {
        source: mimirsbrunn::osm_reader::Error,
    },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Street Extraction from OSM PBF Error {}", source))]
    StreetOsmExtraction {
        source: mimirsbrunn::osm_reader::street::Error,
    },

    #[snafu(display("Street Index Creation Error {}", source))]
    StreetIndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Poi Extraction from OSM PBF Error {}", source))]
    PoiOsmExtraction {
        source: mimirsbrunn::osm_reader::poi::Error,
    },

    #[snafu(display("Poi Index Creation Error {}", source))]
    PoiIndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Elasticsearch Configuration {}", source))]
    StreetElasticsearchConfiguration { source: common::config::Error },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = settings::Opts::from_args();

    let settings = settings::Settings::new(&opts)
        .and_then(settings::validate)
        .context(Settings)?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::wrapped_launch_async(
            &settings.logging.path.clone(),
            move || run(opts, settings),
        )
        .await
        .context(Execution),
        settings::Command::Config => {
            println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            Ok(())
        }
    }
}

const POI_REVERSE_GEOCODING_CONCURRENCY: usize = 8;

async fn run(
    opts: settings::Opts,
    settings: settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut osm_reader =
        mimirsbrunn::osm_reader::make_osm_reader(&opts.input).context(OsmPbfReader)?;

    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch.clone())
        .await
        .context(ElasticsearchConnection)?;

    let admins_geofinder: AdminGeoFinder = match client.list_documents().await {
        Ok(stream) => {
            stream
                .map(|admin| admin.expect("could not parse admin"))
                .collect()
                .await
        }
        Err(err) => {
            warn!(
                "administratives regions not found in Elasticsearch. {:?}",
                err
            );
            std::iter::empty().collect()
        }
    };

    if settings.streets.import {
        let config = common::config::load_es_config_for::<Street>(
            opts.settings
                .iter()
                .filter_map(|s| {
                    if s.starts_with("elasticsearch.street") {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            settings.container.dataset.clone(),
        )
        .context(StreetElasticsearchConfiguration)?;

        let streets = streets(
            &mut osm_reader,
            &admins_geofinder,
            &settings.streets.exclusions,
            #[cfg(feature = "db-storage")]
            settings.database.as_ref(),
        )
        .context(StreetOsmExtraction)?;

        import_streets(streets, &client, config).await?;
    }

    if settings.pois.import {
        let config = common::config::load_es_config_for::<Poi>(
            opts.settings
                .iter()
                .filter_map(|s| {
                    if s.starts_with("elasticsearch.poi") {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            settings.container.dataset.clone(),
        )
        .context(StreetElasticsearchConfiguration)?;

        import_pois(
            &mut osm_reader,
            &admins_geofinder,
            &settings.pois.config.clone().unwrap_or_default(),
            &client,
            config,
        )
        .await?;
    }

    Ok(())
}

#[instrument(skip_all)]
async fn import_streets(
    streets: Vec<places::street::Street>,
    client: &ElasticsearchStorage,
    config: Config,
) -> Result<(), Error> {
    let streets = streets
        .into_iter()
        .map(|street| street.set_weight_from_admins());

    let _index = client
        .generate_index(
            config,
            futures::stream::iter(streets),
            IndexVisibility::Public,
        )
        .await
        .context(StreetIndexCreation)?;

    Ok(())
}

#[instrument(skip_all)]
async fn import_pois(
    osm_reader: &mut mimirsbrunn::osm_reader::OsmPbfReader,
    admins_geofinder: &AdminGeoFinder,
    poi_config: &mimirsbrunn::osm_reader::poi::PoiConfig,
    client: &ElasticsearchStorage,
    config: Config,
) -> Result<(), Error> {
    let pois = mimirsbrunn::osm_reader::poi::pois(osm_reader, poi_config, admins_geofinder)
        .context(PoiOsmExtraction)?;

    let pois: Vec<places::poi::Poi> = futures::stream::iter(pois)
        .map(mimirsbrunn::osm_reader::poi::compute_weight)
        .map(|poi| mimirsbrunn::osm_reader::poi::add_address(client, poi))
        .buffer_unordered(POI_REVERSE_GEOCODING_CONCURRENCY)
        .collect()
        .await;

    let _ = client
        .generate_index(config, futures::stream::iter(pois), IndexVisibility::Public)
        .await
        .context(PoiIndexCreation)?;

    Ok(())
}

// // We need to allow for unused variables, because currently all the checks on
// // args require the db-storage feature. If this feature is not used, then there
// // is a warning
// #[allow(unused_variables)]
// fn validate_args(args: &Args) -> Result<(), mimirsbrunn::Error> {
//     #[cfg(feature = "db-storage")]
//     if args.db_file.is_some() {
//         // If the user specified db_file, he must also specify db_buffer_size, or else!
//         if args.db_buffer_size.is_none() {
//             return Err(failure::format_err!("You need to specify database buffer size if you want to use database storage. Use --db-buffer-size"));
//         }
//     }
//     #[cfg(feature = "db-storage")]
//     if args.db_buffer_size.is_some() {
//         // If the user specified db_buffer_size, he must also specify db_file, or else!
//         if args.db_file.is_none() {
//             return Err(failure::format_err!("You need to specify database file if you want to use database storage. Use --db-file"));
//         }
//     }
//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::elasticsearch::ElasticsearchStorage;
//     use common::config::load_es_config_for;
//     use common::document::ContainerDocument;
//     use futures::TryStreamExt;
//     use mimir::domain::model::query::Query;
//     use mimir::domain::ports::primary::list_documents::ListDocuments;
//     use mimir::domain::ports::primary::search_documents::SearchDocuments;
//     use mimir::{adapters::secondary::elasticsearch::remote, utils::docker};
//     use mimirsbrunn::admin::index_cosmogony;
//     use places::{admin::Admin, street::Street, Place};
//     use serial_test::serial;
//     use structopt::StructOpt;
//
//     fn elasticsearch_test_url() -> String {
//         std::env::var(elasticsearch::remote::ES_TEST_KEY).expect("env var")
//     }
//
//     async fn index_cosmogony_admins(client: &ElasticsearchStorage) {
//         index_cosmogony(
//             "./tests/fixtures/cosmogony.json".into(),
//             vec![],
//             load_es_config_for(
//                 None,
//                 None,
//                 vec!["container.dataset=osm2mimir-test".into()],
//                 String::from("fr"),
//             )
//             .unwrap(),
//             client,
//         )
//         .await
//         .unwrap()
//     }
//
//     #[tokio::test]
//     #[serial]
//     async fn should_correctly_index_osm_streets_and_pois() {
//         docker::initialize()
//             .await
//             .expect("elasticsearch docker initialization");
//
//         // Now we query the index we just created. Since it's a small cosmogony file with few entries,
//         // we'll just list all the documents in the index, and check them.
//         let pool = remote::connection_test_pool()
//             .await
//             .expect("Elasticsearch Connection Pool");
//
//         let client = pool
//             .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
//             .await
//             .expect("Elasticsearch Connection Established");
//
//         index_cosmogony_admins(&client).await;
//
//         let storage_args = if cfg!(feature = "db-storage") {
//             vec!["--db-file=test-db.sqlite3", "--db-buffer-size=10"]
//         } else {
//             vec![]
//         };
//
//         let args = Args::from_iter(
//             [
//                 "osm2mimir",
//                 "--input=./tests/fixtures/osm_fixture.osm.pbf",
//                 "--dataset=osm2mimir-test",
//                 "--import-way=true",
//                 "--import-poi=true",
//                 &format!("-c={}", elasticsearch_test_url()),
//             ]
//             .iter()
//             .copied()
//             .chain(storage_args),
//         );
//
//         let _res = mimirsbrunn::utils::launch_async_args(run, args).await;
//
//         let search = |query: &str| {
//             let client = client.clone();
//             let query: String = query.into();
//             async move {
//                 client
//                     .search_documents(
//                         vec![
//                             Street::static_doc_type().into(),
//                             Poi::static_doc_type().into(),
//                         ],
//                         Query::QueryString(format!("full_label.prefix:({})", query)),
//                     )
//                     .await
//                     .unwrap()
//                     .into_iter()
//                     .map(|json| serde_json::from_value::<Place>(json).unwrap())
//                     .collect::<Vec<Place>>()
//             }
//         };
//
//         let streets: Vec<Street> = client
//             .list_documents()
//             .await
//             .unwrap()
//             .try_collect()
//             .await
//             .unwrap();
//         assert_eq!(streets.len(), 13);
//
//         // Basic street search
//         let results = search("Rue des Près").await;
//         assert_eq!(results[0].label(), "Rue des Près (Livry-sur-Seine)");
//         assert_eq!(
//             results
//                 .iter()
//                 .filter(
//                     |place| place.is_street() && place.label() == "Rue des Près (Livry-sur-Seine)"
//                 )
//                 .count(),
//             1,
//             "Only 1 'Rue des Près' is expected"
//         );
//
//         // All ways with same name in the same city are merged into a single street
//         let results = search("Rue du Four à Chaux").await;
//         assert_eq!(
//             results.iter()
//                 .filter(|place| place.label() == "Rue du Four à Chaux (Livry-sur-Seine)")
//                 .count(),
//             1,
//             "Only 1 'Rue du Four à Chaux' is expected as all ways the same name should be merged into 1 street."
//         );
//         assert_eq!(
//             results[0].id(),
//             "street:osm:way:40812939",
//             "The way with minimum way_id should be used as street id."
//         );
//
//         // Street admin is based on a middle node.
//         // (Here the first node is located outside Melun)
//         let results = search("Rue Marcel Houdet").await;
//         assert_eq!(results[0].label(), "Rue Marcel Houdet (Melun)");
//         assert!(results[0]
//             .admins()
//             .iter()
//             .filter(|a| a.is_city())
//             .any(|a| a.name == "Melun"));
//
//         // Basic search for Poi by label
//         let res = search("Le-Mée-sur-Seine Courtilleraies").await;
//         assert_eq!(
//             res[0].poi().expect("Place should be a poi").poi_type.id,
//             "poi_type:amenity:post_office"
//         );
//
//         // highway=bus_stop should not be indexed
//         let res = search("Grand Châtelet").await;
//         assert!(
//             res.is_empty(),
//             "'Grand Châtelet' (highway=bus_stop) should not be found."
//         );
//
//         // "Rue de Villiers" is at the exact neighborhood between two cities, a
//         // document must be added for both.
//         let results = search("Rue de Villiers").await;
//         assert!(["Neuilly-sur-Seine", "Levallois-Perret"]
//             .iter()
//             .all(|city| {
//                 results.iter().any(|poi| {
//                     poi.admins()
//                         .iter()
//                         .filter(|a| a.is_city())
//                         .any(|admin| &admin.name == city)
//                 })
//             }));
//     }
// }
