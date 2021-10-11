use geo_types::{Coordinate, MultiPolygon, Rect};
use geojson::Geometry;
use serde::{Deserialize, Serialize};
use tracing::warn;

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
                    .set_default("container.dataset", "default")
                    .unwrap()
                    .set_default("elasticsearch.parameters.timeout", "10s")
                    .unwrap()
                    .set_default("elasticsearch.parameters.wait_for_active_shards", "1")
                    .unwrap()
                    .add_source(config::File::from_str(
                        include_str!(concat!(
                            "../../../config/elasticsearch/",
                            $doc_type,
                            "/settings.json"
                        )),
                        config::FileFormat::Json,
                    ))
                    .add_source(config::File::from_str(
                        include_str!(concat!(
                            "../../../config/elasticsearch/",
                            $doc_type,
                            "/mappings.json"
                        )),
                        config::FileFormat::Json,
                    ))
                    .add_source(config::File::from_str(
                        include_str!("../../../config/elasticsearch/synonyms_settings.json"),
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

pub fn serialize_rect<S>(bbox: &Option<Rect<f64>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match bbox {
        Some(b) => {
            // bbox serialized as an array
            // using GeoJSON bounding box format
            // See RFC 7946: https://tools.ietf.org/html/rfc7946#section-5
            let geojson_bbox: geojson::Bbox = vec![b.min().x, b.min().y, b.max().x, b.max().y];
            geojson_bbox.serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}

pub fn deserialize_rect<'de, D>(d: D) -> Result<Option<Rect<f64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<f64>>::deserialize(d).map(|option| {
        option.map(|b| {
            Rect::new(
                Coordinate { x: b[0], y: b[1] }, // min
                Coordinate { x: b[2], y: b[3] }, // max
            )
        })
    })
}

pub fn custom_multi_polygon_serialize<S>(
    multi_polygon_option: &Option<MultiPolygon<f64>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use geojson::{GeoJson, Value};

    match *multi_polygon_option {
        Some(ref multi_polygon) => {
            GeoJson::Geometry(Geometry::new(Value::from(multi_polygon))).serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}

pub fn custom_multi_polygon_deserialize<'de, D>(d: D) -> Result<Option<MultiPolygon<f64>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use std::convert::TryInto;

    Option::<geojson::GeoJson>::deserialize(d).map(|option| {
        option.and_then(|geojson| match geojson {
            geojson::GeoJson::Geometry(geojson_geometry) => {
                let res: Result<MultiPolygon<f64>, _> = geojson_geometry.value.try_into();
                match res {
                    Ok(multi_polygon) => Some(multi_polygon),
                    Err(err) => {
                        warn!("Cannot deserialize into MultiPolygon: {}", err);
                        None
                    }
                }
            }
            _ => None,
        })
    })
}
