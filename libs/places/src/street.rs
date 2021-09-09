use common::document::{ContainerDocument, Document};
use geojson::Geometry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::admin::Admin;
use super::context::Context;
use super::coord::Coord;
use super::Members;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(tag = "type", rename = "street")]
pub struct Street {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub administrative_regions: Vec<Arc<Admin>>,
    pub label: String,
    pub weight: f64,
    /// coord used for some geograhic queries in ES, less precise but  faster than `coord`
    /// https://www.elastic.co/guide/en/elasticsearch/reference/2.4/geo-shape.html
    #[serde(skip_deserializing)]
    pub approx_coord: Option<Geometry>,
    pub coord: Coord,
    pub zip_codes: Vec<String>,
    #[serde(default)]
    pub country_codes: Vec<String>,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,

    pub context: Option<Context>,
}

impl Incr for Street {
    fn id(&self) -> &str {
        &self.id
    }
    fn incr(&mut self) {
        self.weight += 1.;
    }
}

impl Members for Street {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        self.administrative_regions.clone()
    }
}

impl Document for Street {
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl ContainerDocument for Street {
    fn static_doc_type() -> &'static str {
        "street"
    }

    fn default_es_settings() -> &'static str {
        include_str!("../../../config/street/settings.json")
    }

    fn default_es_mappings() -> &'static str {
        include_str!("../../../config/street/mappings.json")
    }
}

pub trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}
