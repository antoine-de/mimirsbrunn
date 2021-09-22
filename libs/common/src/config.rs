use crate::document::ContainerDocument;
use config::Config;
use std::path::PathBuf;

fn config_from_args(
    args: impl IntoIterator<Item = String>,
) -> Result<Config, Box<dyn std::error::Error>> {
    let mut config = Config::builder();

    for arg in args {
        let (key, val) = arg
            .split_once('=')
            .ok_or("missing '=' in setting override syntax")?;

        config = {
            if let Ok(as_bool) = val.parse::<bool>() {
                config.set_override(key, as_bool)
            } else if let Ok(as_int) = val.parse::<i64>() {
                config.set_override(key, as_int)
            } else if let Ok(as_float) = val.parse::<f64>() {
                config.set_override(key, as_float)
            } else {
                config.set_override(key, val)
            }
        }?
    }

    Ok(config.build()?)
}

pub fn load_es_config_for<D: ContainerDocument>(
    mappings: Option<PathBuf>,
    settings: Option<PathBuf>,
    args_override: Vec<String>,
) -> Result<Config, Box<dyn std::error::Error>> {
    let mut cfg_builder = Config::builder().add_source(D::default_es_container_config());

    if let Some(mappings) = mappings {
        cfg_builder = cfg_builder.add_source(config::File::from(mappings))
    }

    if let Some(settings) = settings {
        cfg_builder = cfg_builder.add_source(config::File::from(settings));
    }

    Ok(cfg_builder
        .add_source(
            config_from_args(args_override)
                .map_err(|err| format!("couldn't override settings: {}", err))?,
        )
        .build()?)
}
