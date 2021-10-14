use config::Config;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorageConfig;
/// This module contains the definition for osm2mimir configuration and command line arguments.
///
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Config Compilation Error: {}", source))]
    ConfigCompilation { source: common::config::Error },
    #[snafu(display("Config Merge Error: {} [{}]", msg, source))]
    ConfigMerge {
        msg: String,
        source: config::ConfigError,
    },
    #[snafu(display("Invalid Configuration: {}", msg))]
    Invalid { msg: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Street {
    pub import: bool,
    pub exclusions: crate::osm_reader::street::StreetExclusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poi {
    pub import: bool,
    pub config: Option<crate::osm_reader::poi::PoiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub dataset: String,
}

#[cfg(feature = "db-storage")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub file: PathBuf,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mode: Option<String>,
    pub logging: Logging,
    pub elasticsearch: ElasticsearchStorageConfig,
    pub pois: Poi,
    pub streets: Street,
    pub container: Container,
    #[cfg(feature = "db-storage")]
    pub database: Option<Database>,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "osm2mimir",
    about = "Parsing OSM PBF document and indexing its content in Elasticsearch",
    version = VERSION,
    author = AUTHORS
    )]
pub struct Opts {
    /// Defines the config directory
    ///
    /// This directory must contain 'elasticsearch' and 'osm2mimir' subdirectories.
    #[structopt(parse(from_os_str), short = "c", long = "config-dir")]
    pub config_dir: PathBuf,

    /// Defines the run mode in {testing, dev, prod, ...}
    ///
    /// If no run mode is provided, a default behavior will be used.
    #[structopt(short = "m", long = "run-mode")]
    pub run_mode: Option<String>,

    /// Override settings values using key=value
    #[structopt(short = "s", long = "setting")]
    pub settings: Vec<String>,

    /// OSM PBF file
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    pub input: PathBuf,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Execute osm2mimir with the given configuration
    Run,
    /// Prints osm2mimir's configuration
    Config,
}

// TODO Parameterize the config directory
impl Settings {
    // Read the configuration from <config-dir>/osm2mimir and <config-dir>/elasticsearch
    pub fn new(opts: &Opts) -> Result<Self, Error> {
        let mut builder = Config::builder();

        builder = builder.add_source(
            common::config::config_from(
                opts.config_dir.as_ref(),
                &["osm2mimir", "elasticsearch"],
                opts.run_mode.as_deref(),
                "OSM2MIMIR",
                opts.settings.clone(),
            )
            .context(ConfigCompilation)?,
        );

        // FIXME depending on service.pois.import and service.streets.import, read the corresponding
        // elasticsearch sub dirs.
        let config = builder.build().context(ConfigMerge {
            msg: String::from("Cannot build the configuration from sources"),
        })?;

        config.try_into().context(ConfigMerge {
            msg: String::from("Cannot convert configuration into osm2mimir settings"),
        })
    }
}

// This function returns an error if the settings are invalid.
pub fn validate(settings: Settings) -> Result<Settings, Error> {
    let import_streets_enabled = settings.streets.import;

    let import_poi_enabled = settings.pois.import;

    if !import_streets_enabled && !import_poi_enabled {
        return Err(Error::Invalid {
            msg: String::from("Neither streets nor POIs import is enabled. Nothing to do. Use -s pois.import=true or -s streets.import=true")
        });
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_ok_with_default_config_dir() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let opts = Opts {
            config_dir,
            run_mode: None,
            settings: vec![],
            cmd: Command::Run,
            input: PathBuf::from("foo.osm.pbf"),
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(settings.unwrap().mode, None);
    }

    #[test]
    fn should_override_elasticsearch_port_with_command_line() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let opts = Opts {
            config_dir,
            run_mode: None,
            settings: vec![String::from("elasticsearch.url=http://localhost:9999")],
            cmd: Command::Run,
            input: PathBuf::from("foo.osm.pbf"),
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(
            settings.unwrap().elasticsearch.url.as_str(),
            "http://localhost:9999/"
        );
    }

    #[test]
    fn should_override_elasticsearch_port_environment_variable() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        std::env::set_var("OSM2MIMIR_ELASTICSEARCH_URL", "http://localhost:9999");
        let opts = Opts {
            config_dir,
            run_mode: None,
            settings: vec![],
            cmd: Command::Run,
            input: PathBuf::from("foo.osm.pbf"),
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(
            settings.unwrap().elasticsearch.url.as_str(),
            "http://localhost:9999/"
        );
    }
}
