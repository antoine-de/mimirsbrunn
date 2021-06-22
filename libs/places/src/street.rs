use geojson::Geometry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::admin::Admin;
use super::context::Context;
use super::coord::Coord;
use super::{Members, MimirObject, PlaceDocType};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

impl MimirObject for Street {
    fn is_geo_data() -> bool {
        true
    }
    fn doc_type() -> &'static str {
        PlaceDocType::Street.as_str()
    }
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
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

pub trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}
