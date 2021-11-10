use futures::stream::{StreamExt, TryStreamExt};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

use common::document::ContainerDocument;
use mimir::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir::domain::model::configuration::root_doctype_dataset;
use mimir::domain::ports::primary::{generate_index::GenerateIndex, list_documents::ListDocuments};
use mimir::domain::ports::secondary::storage::{Error as StorageError, Storage};
use mimirsbrunn::admin_geofinder::AdminGeoFinder;
use places::poi::Poi;
use places::street::Street;

const POI_REVERSE_GEOCODING_CONCURRENCY: usize = 8;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Indexing Error: {}", details))]
    Indexing { details: String },

    #[snafu(display("Container Search Error: {}", source))]
    ContainerSearch { source: StorageError },

    #[snafu(display("OSM PBF Reader Error: {}", source))]
    OsmPbfReader {
        source: mimirsbrunn::osm_reader::Error,
    },
    #[snafu(display("Poi Extraction from OSM PBF Error {}", source))]
    PoiOsmExtraction {
        source: mimirsbrunn::osm_reader::poi::Error,
    },

    #[snafu(display("Street Extraction from OSM PBF Error {}", source))]
    StreetOsmExtraction {
        source: mimirsbrunn::osm_reader::street::Error,
    },

    #[snafu(display("Poi Index Creation Error {}", source))]
    PoiIndexCreation {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("List Document Error {}", source))]
    ListDocument {
        source: mimir::domain::model::error::Error,
    },

    #[snafu(display("Could not get Config {}", source))]
    Config { source: common::config::Error },

    #[snafu(display("Invalid Configuration {}", source))]
    ConfigInvalid { source: config::ConfigError },
}

pub enum Status {
    Skipped,
    Done,
}

pub async fn index_pois(
    client: &ElasticsearchStorage,
    region: &str,
    dataset: &str,
    reindex_if_already_exists: bool,
) -> Result<Status, Error> {
    // Check if the address index already exists
    let container = root_doctype_dataset(Poi::static_doc_type(), dataset);

    let index = client
        .find_container(container)
        .await
        .context(ContainerSearch)?;

    // If the previous step has been skipped, then we don't need to index BANO file.
    if index.is_some() && !reindex_if_already_exists {
        return Ok(Status::Skipped);
    }

    let base_path = env!("CARGO_MANIFEST_DIR");
    let input_dir: PathBuf = [base_path, "..", "..", "tests", "fixtures", "osm", region]
        .iter()
        .collect();
    let input_file = input_dir.join(format!("{}-latest.osm.pbf", region));

    let mut osm_reader =
        mimirsbrunn::osm_reader::make_osm_reader(&input_file).context(OsmPbfReader)?;

    let admins_geofinder: AdminGeoFinder = client
        .list_documents()
        .await
        .context(ListDocument)?
        .try_collect()
        .await
        .context(ListDocument)?;

    // Read the poi configuration from the osm2mimir configuration / testing mode.
    let base_path = env!("CARGO_MANIFEST_DIR");
    let config_dir: PathBuf = [base_path, "..", "..", "config"].iter().collect();
    let config: mimirsbrunn::settings::osm2mimir::Settings = common::config::config_from(
        &config_dir,
        &["osm2mimir", "elasticsearch", "logging"],
        "testing",
        None,
        vec![],
    )
    .context(Config)?
    .try_into()
    .context(ConfigInvalid)?;

    let pois = mimirsbrunn::osm_reader::poi::pois(
        &mut osm_reader,
        &config.pois.config.unwrap(),
        &admins_geofinder,
    )
    .context(PoiOsmExtraction)?;

    let pois: Vec<Poi> = futures::stream::iter(pois)
        .map(mimirsbrunn::osm_reader::poi::compute_weight)
        .map(|poi| mimirsbrunn::osm_reader::poi::add_address(client, poi))
        .buffer_unordered(POI_REVERSE_GEOCODING_CONCURRENCY)
        .collect()
        .await;
    let _ = client
        .generate_index(&config.container_poi, futures::stream::iter(pois))
        .await
        .context(PoiIndexCreation)?;

    Ok(Status::Done)
}

pub async fn index_streets(
    client: &ElasticsearchStorage,
    region: &str,
    dataset: &str,
    reindex_if_already_exists: bool,
) -> Result<Status, Error> {
    // Check if the address index already exists
    let container = root_doctype_dataset(Street::static_doc_type(), dataset);

    let index = client
        .find_container(container)
        .await
        .context(ContainerSearch)?;

    // If the previous step has been skipped, then we don't need to index OSM file.
    if index.is_some() && !reindex_if_already_exists {
        return Ok(Status::Skipped);
    }

    let base_path = env!("CARGO_MANIFEST_DIR");
    let input_dir: PathBuf = [base_path, "..", "..", "tests", "fixtures", "osm", region]
        .iter()
        .collect();
    let input_file = input_dir.join(format!("{}-latest.osm.pbf", region));

    let mut osm_reader =
        mimirsbrunn::osm_reader::make_osm_reader(&input_file).context(OsmPbfReader)?;

    let admins_geofinder: AdminGeoFinder = client
        .list_documents()
        .await
        .context(ListDocument)?
        .try_collect()
        .await
        .context(ListDocument)?;

    // Read the street configuration from the osm2mimir configuration / testing mode.
    let base_path = env!("CARGO_MANIFEST_DIR");
    let config_dir: PathBuf = [base_path, "..", "..", "config"].iter().collect();
    let config: mimirsbrunn::settings::osm2mimir::Settings = common::config::config_from(
        &config_dir,
        &["osm2mimir", "elasticsearch", "logging"],
        "testing",
        None,
        vec![],
    )
    .context(Config)?
    .try_into()
    .context(ConfigInvalid)?;

    let streets: Vec<Street> = mimirsbrunn::osm_reader::street::streets(
        &mut osm_reader,
        &admins_geofinder,
        &config.streets.exclusions,
    )
    .context(StreetOsmExtraction)?
    .into_iter()
    .map(|street| street.set_weight_from_admins())
    .collect();

    let _ = client
        .generate_index(&config.container_street, futures::stream::iter(streets))
        .await
        .context(PoiIndexCreation)?;

    Ok(Status::Done)
}
