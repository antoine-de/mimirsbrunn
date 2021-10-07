use common::document::Document;
use cosmogony::ZoneType;
use geo_types::{MultiPolygon, Rect};
use geojson::Geometry;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::context::Context;
use super::coord::Coord;
use super::i18n_properties::I18nProperties;
use super::utils::{
    custom_multi_polygon_deserialize, custom_multi_polygon_serialize, deserialize_rect,
    serialize_rect,
};
use super::Members;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(tag = "type", rename = "admin")]
pub struct Admin {
    pub id: String,
    pub insee: String,
    pub level: u32,
    pub label: String,
    pub name: String,
    pub zip_codes: Vec<String>,
    pub weight: f64,
    /// coord used for some geograhic queries in ES, less precise but  faster than `coord`
    /// https://www.elastic.co/guide/en/elasticsearch/reference/2.4/geo-shape.html
    #[serde(skip_deserializing)]
    pub approx_coord: Option<Geometry>,
    pub coord: Coord,
    #[serde(
        serialize_with = "custom_multi_polygon_serialize",
        deserialize_with = "custom_multi_polygon_deserialize",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub boundary: Option<MultiPolygon<f64>>,
    #[serde(default)]
    pub administrative_regions: Vec<Arc<Admin>>,

    #[serde(
        serialize_with = "serialize_rect",
        deserialize_with = "deserialize_rect",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub bbox: Option<Rect<f64>>,

    #[serde(default)]
    pub zone_type: Option<ZoneType>,
    #[serde(default)]
    pub parent_id: Option<String>, // id of the Admin's parent (from the cosmogony's hierarchy)
    #[serde(default)]
    pub country_codes: Vec<String>,

    #[serde(default)]
    pub codes: BTreeMap<String, String>,

    #[serde(default)]
    pub names: I18nProperties,

    #[serde(default)]
    pub labels: I18nProperties,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,

    pub context: Option<Context>,
}

impl Admin {
    pub fn is_city(&self) -> bool {
        matches!(self.zone_type, Some(ZoneType::City))
    }
}

impl Document for Admin {
    fn id(&self) -> String {
        self.id.clone()
    }
}

crate::impl_container_document!(Admin, "admin");

impl Ord for Admin {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Admin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Admin {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Members for Admin {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        vec![Arc::new(self.clone())]
    }
}

impl Eq for Admin {}

impl From<&Admin> for geojson::Geometry {
    fn from(admin: &Admin) -> Self {
        geojson::Geometry::from(admin.coord)
    }
}
