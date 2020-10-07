use config::{Config, File};
use failure::ResultExt;
use serde::Deserialize;
use std::path::Path;

use crate::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct StreetExclusion {
    pub highways: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Street {
    pub exclusion: StreetExclusion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Poi {}

#[derive(Debug, Clone, Deserialize)]
pub struct Admin {}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub street: Option<Street>,
    pub poi: Option<Poi>,
    pub admin: Option<Admin>,
}

impl Settings {
    pub fn new(config_dir: &Path, name: &Option<String>) -> Result<Self, Error> {
        let mut config = Config::new();

        let default_path = config_dir.join("default");
        // Start off by merging in the "default" configuration file

        if let Some(path) = default_path.to_str() {
            config.merge(File::with_name(path)).with_context(|e| {
                format!(
                    "Could not merge default configuration from file {}: {}",
                    path, e
                )
            })?;
        } else {
            return Err(failure::err_msg(format!(
                "Could not read default settings in '{}'",
                default_path.display()
            )));
        }

        // If we provided a special configuration, merge it.
        if let Some(name) = name {
            let name_path = config_dir.join(name);

            if let Some(path) = name_path.to_str() {
                config
                    .merge(File::with_name(path).required(true))
                    .with_context(|e| {
                        format!(
                            "Could not merge {} configuration in file {}: {}",
                            name, path, e
                        )
                    })?;
            } else {
                return Err(failure::err_msg(format!(
                    "Could not read {} settings in '{}'",
                    name,
                    name_path.display()
                )));
            }
        }

        // You can deserialize (and thus freeze) the entire configuration as
        config.try_into().map_err(|e| {
            failure::err_msg(format!(
                "Could not generate settings from configuration: {}",
                e
            ))
        })
    }
}
