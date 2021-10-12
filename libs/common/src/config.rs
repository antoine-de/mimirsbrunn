use crate::document::ContainerDocument;
use config::{Config, Environment, File};
use snafu::{ResultExt, Snafu};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Key Value Splitting Error: {}", msg))]
    Splitting { msg: String },

    #[snafu(display("Setting Config Value Error: {}", source))]
    ConfigValue { source: config::ConfigError },

    #[snafu(display("Config Compilation Error: {}", source))]
    ConfigCompilation { source: config::ConfigError },
}

/// Create a new configuration source from a list of assignments key=value
///
/// The function iterates over the list, and for each element, it tries to
/// (a) identify the key and the value, by searching for the '=' sign.
/// (b) parse the value into one of bool, i64, f64. if not it's a string.
fn config_from_args(args: impl IntoIterator<Item = String>) -> Result<Config, Error> {
    let mut config = Config::builder();

    for arg in args {
        let (key, val) = arg.split_once('=').ok_or(Error::Splitting {
            msg: format!("missing '=' in setting override: {}", arg),
        })?;

        config = {
            if let Ok(as_bool) = val.parse::<bool>() {
                config.set_override(key, as_bool).context(ConfigValue)
            } else if let Ok(as_int) = val.parse::<i64>() {
                config.set_override(key, as_int).context(ConfigValue)
            } else if let Ok(as_float) = val.parse::<f64>() {
                config.set_override(key, as_float).context(ConfigValue)
            } else {
                config.set_override(key, val).context(ConfigValue)
            }
        }?
    }

    config.build().context(ConfigCompilation)
}

pub fn load_es_config_for<D: ContainerDocument>(
    mappings: Option<PathBuf>,
    settings: Option<PathBuf>,
    args_override: Vec<String>,
    dataset: String,
) -> Result<Config, Error> {
    let mut cfg_builder = Config::builder().add_source(D::default_es_container_config());

    let config_dataset = config::Config::builder()
        .set_override("container.dataset", dataset)
        .unwrap()
        .build()
        .context(ConfigCompilation)?;

    cfg_builder = cfg_builder.add_source(config_dataset);

    if let Some(mappings) = mappings {
        cfg_builder = cfg_builder.add_source(config::File::from(mappings))
    }

    if let Some(settings) = settings {
        cfg_builder = cfg_builder.add_source(config::File::from(settings));
    }

    cfg_builder
        .add_source(config_from_args(args_override)?)
        .build()
        .context(ConfigCompilation)
}

// This function produces a new configuration based on the arguments:
// In the base directory (`config_dir`), it will look for the subdirectories.
// * In each subdirectory, it will look for a default configuration file.
// * If a run_mode is provided, then the corresponding file is sourced in each of these
//   subdirectory.
// * Then, if a prefix is given, we source environment variables starting with that string.
// * And finally, we can make manual adjusts with a list of key value pairs.
pub fn config_from<
    'a,
    T: Into<Option<String>> + Clone,
    U: IntoIterator<Item = String>,
    V: Into<Option<&'a str>>,
>(
    config_dir: &Path,
    sub_dirs: &[&str],
    run_mode: T,
    prefix: V,
    settings: U,
) -> Result<Config, Error> {
    let mut builder = sub_dirs
        .iter()
        .fold(Config::builder(), |mut builder, sub_dir| {
            let dir_path = config_dir.join(sub_dir);

            let default_path = dir_path.join("default").with_extension("toml");
            builder = builder.add_source(File::from(default_path));

            // The RUN_MODE environment variable overides the one given as argument:
            if let Some(run_mode) = env::var("RUN_MODE")
                .ok()
                .or_else(|| run_mode.clone().into())
            {
                let run_mode_path = dir_path.join(&run_mode).with_extension("toml");
                builder = builder.add_source(File::from(run_mode_path).required(false));
            }

            // Add in a local configuration file
            // This file shouldn't be checked in to git
            let local_path = dir_path.join("local").with_extension("toml");
            builder = builder.add_source(File::from(local_path).required(false));
            builder
        });

    // Add in settings from the environment (with a prefix of OMS2MIMIR)
    // Eg.. `<prefix>_DEBUG=1 ./target/app` would set the `debug` key
    builder = if let Some(prefix) = prefix.into() {
        builder.add_source(Environment::with_prefix(prefix).separator("_"))
    } else {
        builder
    };

    // Add command line overrides
    builder = builder.add_source(config_from_args(settings)?);

    builder.build().context(ConfigCompilation)
}
