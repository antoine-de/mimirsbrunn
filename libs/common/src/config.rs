use crate::document::ContainerDocument;
use config::Config;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

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
pub fn config_from_args(args: impl IntoIterator<Item = String>) -> Result<Config, Error> {
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

    Ok(config.build().context(ConfigCompilation)?)
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

    Ok(cfg_builder
        .add_source(config_from_args(args_override)?)
        .build()
        .context(ConfigCompilation)?)
}
