use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use snafu::Snafu;
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;

use common::config::config_from_args;
use mimir2::adapters::primary::common::settings::QuerySettings;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Elasticsearch {
    pub host: String,
    pub port: u16,
    pub version_req: String,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Host on which we expose bragi. Example: 'http://localhost', '0.0.0.0'
    pub host: String,
    /// Port on which we expose bragi.
    pub port: u16,
    /// Used on POST request to set an upper limit on the size of the body (in bytes)
    pub content_length_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mode: String,
    pub logging: Logging,
    pub elasticsearch: Elasticsearch,
    pub query: QuerySettings,
    pub service: Service,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "bragi",
    about = "REST API for querying Elasticsearch",
    version = VERSION,
    author = AUTHORS
    )]
pub struct Opts {
    /// Defines the config directory
    #[structopt(parse(from_os_str), short = "c", long = "config-dir")]
    pub config_dir: PathBuf,

    /// Defines the run mode in {testing, dev, prod, ...}
    #[structopt(short = "m", long = "run-mode")]
    pub run_mode: String,

    /// Defines settings values
    #[structopt(short = "s", long = "setting")]
    settings: Vec<String>,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Run,
    Test,
    Config {
        #[structopt(short = "s", long = "setting")]
        setting: Option<String>,
    },
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
    // pub fn new<'a, T: Into<Option<&'a ArgMatches<'a>>>>(matches: T) -> Result<Self, Error> {
    pub fn new(opts: &Opts) -> Result<Self, Error> {
        let bragi_config_dir = opts.config_dir.join("bragi");

        let mut builder = Config::builder();

        let default_path = bragi_config_dir.join("default").with_extension("toml");

        // Start off by merging in the "default" configuration file
        builder = builder.add_source(File::from(default_path));

        // We use the RUN_MODE environment variable, and if not, the command line
        // argument.
        let settings = env::var("RUN_MODE").unwrap_or_else(|_| opts.run_mode.clone());

        let settings_path = bragi_config_dir.join(&settings).with_extension("toml");

        builder = builder.add_source(File::from(settings_path).required(true));

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        // FIXME Check it works
        builder = builder.add_source(File::with_name("config/local").required(false));

        builder =
            builder.add_source(config_from_args(opts.settings.clone()).expect("from settings"));

        // Add in settings from the environment (with a prefix of BRAGI)
        // Eg.. `BRAGI_DEBUG=1 ./target/app` would set the `debug` key
        builder = builder.add_source(Environment::with_prefix("BRAGI").separator("_"));

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

        // The query is stored in a separate config file.
        // FIXME Here we assume the query is stored in query/default.toml, but
        // we should merge also a query/[RUN_MODE].toml to override the default.
        let query_path = opts
            .config_dir
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_ok_with_default_config_dir() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let opts = Opts {
            config_dir,
            run_mode: String::from("testing"),
            settings: vec![],
            cmd: Command::Run,
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(settings.unwrap().mode, String::from("testing"));
    }

    #[test]
    fn should_override_elasticsearch_port_with_command_line() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let opts = Opts {
            config_dir,
            run_mode: String::from("testing"),
            settings: vec![String::from("elasticsearch.port=9999")],
            cmd: Command::Run,
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(settings.unwrap().elasticsearch.port, 9999);
    }

    #[test]
    fn should_override_elasticsearch_port_environment_variable() {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        std::env::set_var("BRAGI_ELASTICSEARCH_PORT", "9999");
        let opts = Opts {
            config_dir,
            run_mode: String::from("testing"),
            settings: vec![],
            cmd: Command::Run,
        };
        let settings = Settings::new(&opts);
        assert!(
            settings.is_ok(),
            "Expected Ok, Got an Err: {}",
            settings.unwrap_err().to_string()
        );
        assert_eq!(settings.unwrap().elasticsearch.port, 9999);
    }
}
