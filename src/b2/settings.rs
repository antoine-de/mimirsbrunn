use clap::ArgMatches;
use config::{Config, Environment, File};
use serde::Deserialize;
use snafu::ResultExt;
use snafu::Snafu;
use std::convert::TryFrom;
use std::env;
use std::path::Path;

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
    pub service: Service,
}

// TODO Parameterize the config directory

impl Settings {
    pub fn new<'a, T: Into<Option<&'a ArgMatches<'a>>>>(matches: T) -> Result<Self, Error> {
        let matches = matches.into().ok_or(Error::ArgMatch {
            msg: String::from("no matches"),
        })?;

        let config_dir = matches.value_of("config dir").ok_or(Error::ArgMissing {
            msg: String::from("no config dir"),
        })?;

        let config_dir = Path::new(config_dir);

        let mut s = Config::new();

        let default_path = config_dir.join("default").with_extension("toml");

        // Start off by merging in the "default" configuration file
        s.merge(File::from(default_path)).context(ConfigMerge {
            msg: String::from("Could not merge default configuration"),
        })?;

        // We use the RUN_MODE environment variable, and if not, the command line
        // argument. If neither is present, we return an error.
        let settings = env::var("RUN_MODE").or_else(|_| {
            matches
                .value_of("settings")
                .ok_or_else(|| Error::ArgMissing {
                    msg: String::from("no settings, you need to set env var RUN_MODE"),
                })
                .map(ToOwned::to_owned)
        })?;

        let settings_path = config_dir.join(&settings).with_extension("toml");

        s.merge(File::from(settings_path).required(true))
            .context(ConfigMerge {
                msg: format!("Could not merge {} configuration", settings),
            })?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false))
            .context(ConfigMerge {
                msg: String::from("Could not merge local configuration"),
            })?;

        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        s.merge(Environment::with_prefix("app"))
            .context(ConfigMerge {
                msg: String::from("Could not merge configuration from environment variables"),
            })?;

        // Now we take care of the elasticsearch.url, which can be had from environment variables.
        let key = match settings.as_str() {
            "testing" => "ELASTICSEARCH_TEST_URL",
            _ => "ELASTICSEARCH_URL",
        };

        if let Ok(es_url) = env::var(key) {
            s.set("elasticsearch.url", es_url).context(ConfigExtract {
                msg: String::from("Could not set elasticsearch url from environment variable"),
            })?;
        }
        // For the port, the value by default is the one in the configuration file. But it
        // gets overwritten by the environment variable STOCKS_GRAPHQL_PORT.
        let default_port = s.get_int("service.port").context(ConfigExtract {
            msg: String::from("Could not get default port"),
        })?;

        // config crate support i64, not u16
        let default_port = u16::try_from(default_port).context(ConfigValue {
            //map_err(|err| error::Error::MiscError {
            msg: String::from("Could not get u16 port"),
        })?;

        // We retrieve the port, first as a string, which must be turned into an u16, because
        // thats a correct port number. But then it should be cast into an i64 in the config,
        // because config doesn't trade in u16s.
        let port = env::var("STOCKS_GRAPHQL_PORT").unwrap_or_else(|_| format!("{}", default_port));

        let port = port.parse::<u16>().context(ConfigParse {
            msg: String::from("Could not parse into a valid port number"),
        })?;

        let port = i64::try_from(port).unwrap(); // infaillible

        s.set("service.port", port).context(ConfigExtract {
            msg: String::from("Could not set service port"),
        })?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into().context(ConfigMerge {
            msg: String::from("Could not generate settings from configuration"),
        })
    }
}
