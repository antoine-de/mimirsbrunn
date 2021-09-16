/// Build default configuration for given place type. By convention this will look in
/// ../../../config/<place_type> for files settings.json and mappings.json.
#[macro_export]
macro_rules! impl_default_es_settings {
    ( $type: ty, $dir_name: literal ) => {
        impl common::container_config::DefaultEsContainerConfig for $type {
            fn default_es_container_config() -> config::Config {
                config::Config::builder()
                    .set_default("container.name", Self::static_doc_type())
                    .unwrap()
                    .set_default("container.dataset", "munin")
                    .unwrap()
                    .set_default("elasticsearch.parameters.timeout", "10s")
                    .unwrap()
                    .set_default("elasticsearch.parameters.wait_for_active_shards", "1")
                    .unwrap()
                    .add_source(config::File::from_str(
                        include_str!(concat!("../../../config/", $dir_name, "/settings.json")),
                        config::FileFormat::Json,
                    ))
                    .add_source(config::File::from_str(
                        include_str!(concat!("../../../config/", $dir_name, "/mappings.json")),
                        config::FileFormat::Json,
                    ))
                    .build()
                    .expect(concat!(
                        "default configuration is invalid for ",
                        stringify!($type)
                    ))
            }
        }
    };
}
