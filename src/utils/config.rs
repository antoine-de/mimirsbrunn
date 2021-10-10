use config::{builder::DefaultState, Config, ConfigBuilder, Environment, File};
use std::env;
use std::path::Path;

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
pub fn config_builder_from<T: Into<Option<String>> + Clone>(
    config_dir: &Path,
    sub_dirs: &[&str],
    run_mode: T,
    prefix: &str,
) -> ConfigBuilder<DefaultState> {
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
                builder = builder.add_source(File::from(run_mode_path).required(true));
            }

            // Add in a local configuration file
            // This file shouldn't be checked in to git
            let local_path = dir_path.join("local").with_extension("toml");
            builder = builder.add_source(File::from(local_path).required(false));
            builder
        });

    // Add in settings from the environment (with a prefix of OMS2MIMIR)
    // Eg.. `OSM2MIMIR_DEBUG=1 ./target/app` would set the `debug` key
    builder = builder.add_source(Environment::with_prefix(prefix).separator("_"));

    builder
}
