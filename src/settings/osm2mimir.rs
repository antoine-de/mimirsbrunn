use config::{Config, ConfigError, File, FileFormat, Source, Value};
use failure::{format_err, ResultExt};
use serde::Deserialize;
use slog_scope::{info, warn};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::osm_reader::poi;
use crate::Error;
use common::config::load_es_config_for;

#[derive(Debug, Clone, Deserialize)]
pub struct StreetExclusion {
    pub highway: Option<Vec<String>>,
    pub public_transport: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Street {
    pub import: bool,
    pub exclusion: StreetExclusion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Admin {
    pub import: bool,
    pub levels: Vec<u32>,
    pub city_level: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Poi {
    pub import: bool,
    pub config: Option<poi::PoiConfig>,
}

#[cfg(feature = "db-storage")]
#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    pub file: PathBuf,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Elasticsearch {
    pub connection_string: String,
    pub insert_thread_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub dataset: String,
    #[cfg(feature = "db-storage")]
    pub database: Option<Database>,
    pub elasticsearch: Elasticsearch,
    pub street: Option<Street>,
    pub poi: Option<Poi>,
    pub admin: Option<Admin>,
}

impl Settings {
    // To create settings, we first retrieve default settings, merge in specific settings if
    // needed, and finally override them with command line arguments.
    pub fn new(args: Args) -> Result<Self, Error> {
        let config_dir = args.config_dir.clone();
        let settings = args.settings.clone();

        let mut builder = Config::builder();

        builder = match config_dir {
            Some(mut dir) => {
                // Start off by merging in the "default" configuration file
                dir.push("osm2mimir-default");

                let builder = if let Some(path) = dir.to_str() {
                    info!("using configuration from {}", path);
                    // Now if the file exists, we read it, otherwise, we
                    // read from the compiled version.
                    if dir.exists() {
                        builder.add_source(File::with_name(path))
                    } else {
                        builder.add_source(File::from_str(
                            include_str!("../../config/osm2mimir-default.toml"),
                            FileFormat::Toml,
                        ))
                    }
                } else {
                    return Err(failure::err_msg(format!(
                        "Could not read default settings in '{}'",
                        dir.display()
                    )));
                };

                dir.pop(); // remove the default

                // If we provided a special configuration, merge it.
                let builder = if let Some(settings) = settings {
                    dir.push(&settings);

                    let builder = if let Some(path) = dir.to_str() {
                        info!("using configuration from {}", path);
                        builder.add_source(File::with_name(path).required(true))
                    } else {
                        return Err(failure::err_msg(format!(
                            "Could not read configuration for '{}'",
                            settings,
                        )));
                    };
                    dir.pop();
                    builder
                } else {
                    builder
                };
                builder
            }
            None => {
                if settings.is_some() {
                    // If the user set the 'settings' at the command line, he should
                    // also have used the 'config_dir' option. So we issue a warning,
                    // and leave with an error because the expected configuration can
                    // not be read.
                    warn!("settings option used without the 'config_dir' option. Please set the config directory with --config-dir.");
                    return Err(failure::err_msg(String::from(
                        "Could not build program settings",
                    )));
                }
                builder = builder.add_source(File::from_str(
                    include_str!("../../config/osm2mimir-default.toml"),
                    FileFormat::Toml,
                ));
                builder
            }
        };

        // Now override with command line values
        builder = builder.add_source(args);
        // .with_context(|e| format!("Could not merge arguments into configuration: {}", e))?;

        // You can deserialize (and thus freeze) the entire configuration as
        builder
            .build()
            .with_context(|e| format!("could not build configuration: {}", e))?
            .try_into()
            .map_err(|e| {
                failure::err_msg(format!(
                    "Could not generate settings from configuration: {}",
                    e
                ))
            })
    }
}

#[derive(StructOpt, Clone, Debug)]
pub struct Args {
    /// OSM PBF file.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    pub input: PathBuf,
    /// Elasticsearch parameters.
    #[structopt(short = "c", long = "connection-string")]
    connection_string: Option<String>,
    /// Import ways.
    #[structopt(short = "w", long = "import-way")]
    import_way: Option<bool>,
    /// Import POIs.
    #[structopt(short = "p", long = "import-poi")]
    import_poi: Option<bool>,
    /// Name of the dataset.
    #[structopt(short = "d", long, default_value = "fr")]
    pub dataset: String,
    /// Mappings file for Street documents
    #[structopt(parse(from_os_str), long = "street-mappings")]
    street_mappings: Option<PathBuf>,
    /// Settings file for Street documents
    #[structopt(parse(from_os_str), long = "street-settings")]
    street_settings: Option<PathBuf>,
    /// Override config for streets import using syntax `key.subkey=val`
    #[structopt(name = "street-setting", long)]
    override_street_settings: Vec<String>,

    /// Mappings file for Poi documents
    #[structopt(parse(from_os_str), long = "poi-mappings")]
    poi_mappings: Option<PathBuf>,
    /// Settings file for Poi documents
    #[structopt(parse(from_os_str), long = "poi-settings")]
    poi_settings: Option<PathBuf>,
    /// Override config for POIs import using syntax `key.subkey=val`
    #[structopt(name = "poi-setting", long)]
    override_poi_settings: Vec<String>,

    /// If you use this option by providing a filename, then we
    /// will use a SQlite database that will be persisted. You
    /// can only do that if osm2mimir was compiled with the
    /// 'db-storage' feature. If you don't provide a value, then
    /// we will use in memory storage.
    #[cfg(feature = "db-storage")]
    #[structopt(long = "db-file", parse(from_os_str))]
    pub db_file: Option<PathBuf>,

    /// DB buffer size.
    #[cfg(feature = "db-storage")]
    #[structopt(long = "db-buffer-size")]
    pub db_buffer_size: Option<usize>,

    /// Number of threads to use to insert into Elasticsearch. Note that Elasticsearch is not able
    /// to handle values that are too high.
    #[structopt(short = "T", long = "nb-insert-threads")]
    nb_insert_threads: Option<usize>,

    /// Path to the config directory
    /// osm2mimir will read the default configuration in there, and maybe
    /// more depending on the settings option.
    /// If no option is given, we'll just read the ./config/osm2mimir-default.toml
    /// at compile time.
    #[structopt(short = "D", long = "config-dir")]
    config_dir: Option<PathBuf>,

    /// Specific configuration, on top of the default ones.
    /// You should provide the basename of the file, eg acme, so that
    /// osm2mimir will use {config-dir}/acme.toml. (Requires config_dir to
    /// be set)
    #[structopt(short = "s", long = "settings")]
    settings: Option<String>,
}

impl Source for Args {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut m = HashMap::new();

        m.insert(
            String::from("dataset"),
            Value::new(None, self.dataset.clone()),
        );

        // WAY
        if let Some(import_way) = self.import_way {
            m.insert(String::from("street.import"), Value::new(None, import_way));
        }

        // POI
        if let Some(import_poi) = self.import_poi {
            m.insert(String::from("poi.import"), Value::new(None, import_poi));
        }

        // ELASTICSEARCH SETTINGS

        if let Some(connection_string) = self.connection_string.clone() {
            m.insert(
                String::from("elasticsearch.connection_string"),
                Value::new(None, connection_string),
            );
        }

        if let Some(nb_insert_threads) = self.nb_insert_threads {
            m.insert(
                String::from("elasticsearch.insert_thread_count"),
                Value::new(
                    None,
                    i64::try_from(nb_insert_threads).map_err(|e| {
                        ConfigError::Message(format!(
                            "Could not convert elasticsearch insert thread count to integer: {}",
                            e
                        ))
                    })?,
                ),
            );
        }

        // DATABASE
        #[cfg(feature = "db-storage")]
        if let Some(db_file) = self.db_file.clone() {
            m.insert(
                String::from("database.file"),
                Value::new(None, db_file.to_str().expect("valid utf-8 filename")),
            );
        }

        #[cfg(feature = "db-storage")]
        if let Some(db_buffer_size) = self.db_buffer_size {
            m.insert(
                String::from("database.buffer_size"),
                Value::new(
                    None,
                    i64::try_from(db_buffer_size).map_err(|e| {
                        ConfigError::Message(format!(
                            "Could not convert database buffer size to integer: {}",
                            e
                        ))
                    })?,
                ),
            );
        }

        Ok(m)
    }
}

impl Args {
    pub fn get_street_config(&self) -> Result<Config, Error> {
        let config = load_es_config_for::<places::street::Street>(
            self.street_mappings.clone(),
            self.street_settings.clone(),
            self.override_street_settings.clone(),
            self.dataset.clone(),
        )
        .map_err(|err| format_err!("could not load street configuration: {}", err))?;

        Ok(config)
    }

    pub fn get_poi_config(&self) -> Result<Config, Error> {
        let config = load_es_config_for::<places::poi::Poi>(
            self.poi_mappings.clone(),
            self.poi_settings.clone(),
            self.override_poi_settings.clone(),
            self.dataset.clone(),
        )
        .map_err(|err| format_err!("could not load poi configuration: {}", err))?;

        Ok(config)
    }
}
