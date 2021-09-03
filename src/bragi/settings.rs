use clap::ArgMatches;
use config::{Config, Environment, File};
use serde::Deserialize;
use snafu::ResultExt;
use snafu::Snafu;
use std::env;
use std::path::{Path, PathBuf};

use mimir2::adapters::primary::common::settings::QuerySettings;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Arg Match Error: {}", msg))]
    ArgMatch { msg: String },
    #[snafu(display("Arg Missing Error: {}", msg))]
    ArgMissing { msg: String },
    #[snafu(display("Env Var Missing Error: {} [{}]", msg, source))]
    EnvVarMissing { msg: String, source: env::VarError },
    #[snafu(display("Config Merge Error: {} [{}]", msg, source))]
    ConfigMerge {
        msg: String,
        source: config::ConfigError,
    },
    #[snafu(display("Config Extract Error: {} [{}]", msg, source))]
    ConfigExtract {
        msg: String,
        source: config::ConfigError,
    },
    #[snafu(display("Config Value Error: {} [{}]", msg, source))]
    ConfigValue {
        msg: String,
        source: std::num::TryFromIntError,
    },
    #[snafu(display("Config Value Error: {} [{}]", msg, source))]
    ConfigParse {
        msg: String,
        source: std::num::ParseIntError,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logging {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Elasticsearch {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub testing: bool,
    pub mode: String,
    pub logging: Logging,
    pub elasticsearch: Elasticsearch,
    pub query: QuerySettings,
    pub service: Service,
}

// TODO Parameterize the config directory
impl Settings {
    // This function produces a new configuration for bragi based on command line arguments,
    // configuration files, and environment variables.
    // * For bragi, up to three configuration files are read. These files are all in a directory
    //   given by the 'config dir' command line argument.
    // * The first configuration file is 'default.toml'
    // * The second depends on the run mode (eg test, dev, prod). The run mode can be specified
    //   either by the command line, or with the RUN_MODE environment variable. Given a run mode,
    //   we look for the corresponding file in the config directory: 'dev' -> 'config/dev.toml'.
    //   Default values are overriden by this mode config file.
    // * Finally we look for a 'config/local.toml' file which can still override previous values.
    // * Any value in a config file can then be overriden by environment variable: For example
    //   to replace service.port, we can specify XXX
    // * There is a special treatment for:
    //   - Elasticsearch URL, which is specified by ELASTICSEARCH_URL or ELASTICSEARCH_TEST_URL
    //   - Bragi's web server's port and listening address can be specified by command line
    //     arguments.
    pub fn new<'a, T: Into<Option<&'a ArgMatches<'a>>>>(matches: T) -> Result<Self, Error> {
        let matches = matches.into().ok_or(Error::ArgMatch {
            msg: String::from("no matches"),
        })?;

        let config_dir = matches.value_of("config dir").ok_or(Error::ArgMissing {
            msg: String::from("no config dir"),
        })?;

        let config_dir = Path::new(config_dir);

        let mut builder = Config::builder();

        let default_path = config_dir.join("default").with_extension("toml");

        // Start off by merging in the "default" configuration file
        builder = builder.add_source(File::from(default_path));

        // We use the RUN_MODE environment variable, and if not, the command line
        // argument. If neither is present, we return an error.
        let settings = env::var("RUN_MODE").or_else(|_| {
            matches
                .value_of("run mode")
                .ok_or_else(|| Error::ArgMissing {
                    msg: String::from(
                        "no run mode, you need to set one at the command line with -m",
                    ),
                })
                .map(ToOwned::to_owned)
        })?;

        let settings_path = config_dir.join(&settings).with_extension("toml");

        builder = builder.add_source(File::from(settings_path).required(true));

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        builder = builder.add_source(File::with_name("config/local").required(false));

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        builder = builder.add_source(Environment::with_prefix("app"));

        // Now we take care of the elasticsearch.url, which can be had from environment variables.
        let key = match settings.as_str() {
            "testing" => "ELASTICSEARCH_TEST_URL",
            _ => "ELASTICSEARCH_URL",
        };

        builder = if let Ok(es_url) = env::var(key) {
            builder
                .set_override("elasticsearch.url", es_url)
                .context(ConfigExtract {
                    msg: String::from("Could not set elasticsearch url from environment variable"),
                })?
        } else {
            builder
        };

        builder = if let Ok(es_url) = env::var("BRAGI_SERVICE_PORT") {
            builder
                .set_override("service.port", es_url)
                .context(ConfigExtract {
                    msg: String::from("Could not set bragi service port from environment variable"),
                })?
        } else {
            builder
        };

        // The query is stored in a separate config file.
        // FIXME Here we assume the query is stored in query/default.toml, but
        // we should merge also a query/[RUN_MODE].toml to override the default.

        let query_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let query_path = query_path
            .join("config")
            .join("query")
            .join("default")
            .with_extension("toml");

        builder = builder.add_source(File::from(query_path));

        let config = builder.build().context(ConfigMerge {
            msg: String::from("foo"),
        })?;

        config.try_into().context(ConfigMerge {
            msg: String::from("Cannot merge bragi settings"),
        })
    }
}
