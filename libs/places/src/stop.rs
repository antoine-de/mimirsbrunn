use common::document::Document;
use geojson::Geometry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use transit_model::objects::Rgb;
use typed_index_collection::Idx;

use super::admin::Admin;
use super::code::Code;
use super::context::Context;
use super::coord::Coord;
use super::Members;
use super::Property;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct CommercialMode {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysicalMode {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Network {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Line {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_mode: Option<CommercialMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    pub physical_modes: Vec<PhysicalMode>,
    #[serde(skip_serializing)]
    pub sort_order: Option<u32>, // we do not serialise this field, it is only used to sort the Lines
}

pub trait FromTransitModel<T> {
    fn from_transit_model(idx: Idx<T>, navitia: &transit_model::Model) -> Self;
}

impl FromTransitModel<transit_model::objects::Line> for Line {
    fn from_transit_model(
        l_idx: Idx<transit_model::objects::Line>,
        navitia: &transit_model::Model,
    ) -> Self {
        let line = &navitia.lines[l_idx];
        Self {
            id: normalize_id("line", &line.id),
            name: line.name.clone(),
            code: line.code.clone(),
            color: line.color.clone(),
            sort_order: line.sort_order,
            text_color: line.text_color.clone(),
            commercial_mode: navitia
                .commercial_modes
                .get(&line.commercial_mode_id)
                .map(|c| CommercialMode {
                    id: normalize_id("commercial_mode", &c.id),
                    name: c.name.clone(),
                }),
            network: navitia.networks.get(&line.network_id).map(|n| Network {
                id: normalize_id("network", &n.id),
                name: n.name.clone(),
            }),
            physical_modes: navitia
                .get_corresponding_from_idx(l_idx)
                .into_iter()
                .map(|p_idx| {
                    let physical_mode = &navitia.physical_modes[p_idx];
                    PhysicalMode {
                        id: normalize_id("physical_mode", &physical_mode.id),
                        name: physical_mode.name.clone(),
                    }
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct FeedPublisher {
    pub id: String,
    pub license: String,
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(tag = "type", rename = "stop")]
pub struct Stop {
    pub id: String,
    pub label: String,
    pub name: String,
    /// coord used for some geograhic queries in ES, less precise but  faster than `coord`
    /// https://www.elastic.co/guide/en/elasticsearch/reference/2.4/geo-shape.html
    #[serde(skip_deserializing)]
    pub approx_coord: Option<Geometry>,
    pub coord: Coord,
    pub administrative_regions: Vec<Arc<Admin>>,
    pub weight: f64,
    pub zip_codes: Vec<String>,
    #[serde(default)]
    pub commercial_modes: Vec<CommercialMode>,
    #[serde(default)]
    pub physical_modes: Vec<PhysicalMode>,
    #[serde(default)]
    pub coverages: Vec<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub codes: Vec<Code>,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default)]
    pub feed_publishers: Vec<FeedPublisher>,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,
    #[serde(default)]
    pub lines: Vec<Line>,
    #[serde(default)]
    pub country_codes: Vec<String>,

    pub context: Option<Context>,
}

impl Members for Stop {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        self.administrative_regions.clone()
    }
}

impl Document for Stop {
    fn id(&self) -> String {
        self.id.clone()
    }
}

crate::impl_container_document!(Stop, "stop");

// This function reformat the id by removing spaces, and prepending a prefix
pub fn normalize_id(prefix: &str, id: &str) -> String {
    match prefix {
        "stop_area" => format!(
            "{}:{}",
            prefix,
            &id.replacen("StopArea:", "", 1).replace(" ", "")
        ),
        _ => format!("{}:{}", prefix, &id.replace(" ", "")),
    }
}
