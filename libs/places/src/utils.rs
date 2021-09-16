/// Build default configuration for given place type. By convention this will look in
/// ../../../config/<doc_type> for files settings.json and mappings.json.
#[macro_export]
macro_rules! impl_container_document {
    ( $type: ty, $doc_type: literal ) => {
        impl common::document::ContainerDocument for $type {
            fn static_doc_type() -> &'static str {
                $doc_type
            }

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
                        include_str!(concat!("../../../config/", $doc_type, "/settings.json")),
                        config::FileFormat::Json,
                    ))
                    .add_source(config::File::from_str(
                        include_str!(concat!("../../../config/", $doc_type, "/mappings.json")),
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
