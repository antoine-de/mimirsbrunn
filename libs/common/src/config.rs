use config::Config;

pub fn config_from_args(
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
