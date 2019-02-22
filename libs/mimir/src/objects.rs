// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io
use cosmogony::ZoneType;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::iter::FromIterator;
use std::rc::Rc;
use std::sync::Arc;

pub trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}

/// Object stored in elastic search
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Place {
    Admin(Admin),
    Street(Street),
    Addr(Addr),
    Poi(Poi),
    Stop(Stop),
}

/// Object stored in elastic search
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Address {
    Street(Street),
    Addr(Addr),
}

impl Place {
    pub fn is_admin(&self) -> bool {
        match *self {
            Place::Admin(_) => true,
            _ => false,
        }
    }
    pub fn is_street(&self) -> bool {
        match *self {
            Place::Street(_) => true,
            _ => false,
        }
    }
    pub fn is_addr(&self) -> bool {
        match *self {
            Place::Addr(_) => true,
            _ => false,
        }
    }
    pub fn is_poi(&self) -> bool {
        match *self {
            Place::Poi(_) => true,
            _ => false,
        }
    }
    pub fn is_stop(&self) -> bool {
        match *self {
            Place::Stop(_) => true,
            _ => false,
        }
    }
    pub fn poi(&self) -> Option<&Poi> {
        match *self {
            Place::Poi(ref poi) => Some(poi),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match *self {
            Place::Admin(ref o) => o.label(),
            Place::Street(ref o) => o.label(),
            Place::Addr(ref o) => o.label(),
            Place::Poi(ref o) => o.label(),
            Place::Stop(ref o) => o.label(),
        }
    }

    pub fn admins(&self) -> Vec<Arc<Admin>> {
        match *self {
            Place::Admin(ref o) => o.admins(),
            Place::Street(ref o) => o.admins(),
            Place::Addr(ref o) => o.admins(),
            Place::Poi(ref o) => o.admins(),
            Place::Stop(ref o) => o.admins(),
        }
    }

    pub fn address(&self) -> Option<Address> {
        match *self {
            Place::Admin(_) => None,
            Place::Street(ref o) => Some(Address::Street(o.clone())),
            Place::Addr(ref o) => Some(Address::Addr(o.clone())),
            Place::Poi(_) => None,
            Place::Stop(_) => None,
        }
    }

    pub fn distance(&self) -> Option<u32> {
        match *self {
            Place::Admin(ref o) => o.distance,
            Place::Street(ref o) => o.distance,
            Place::Addr(ref o) => o.distance,
            Place::Poi(ref o) => o.distance,
            Place::Stop(ref o) => o.distance,
        }
    }

    pub fn set_distance(&mut self, d: u32) {
        match self {
            Place::Admin(ref mut o) => o.distance = Some(d),
            Place::Street(ref mut o) => o.distance = Some(d),
            Place::Addr(ref mut o) => o.distance = Some(d),
            Place::Poi(ref mut o) => o.distance = Some(d),
            Place::Stop(ref mut o) => o.distance = Some(d),
        }
    }

    pub fn coord(&self) -> &Coord {
        match self {
            Place::Admin(ref o) => &o.coord,
            Place::Street(ref o) => &o.coord,
            Place::Addr(ref o) => &o.coord,
            Place::Poi(ref o) => &o.coord,
            Place::Stop(ref o) => &o.coord,
        }
    }
}

pub trait MimirObject: serde::Serialize {
    fn is_geo_data() -> bool;
    fn doc_type() -> &'static str; // provides the elasticsearch type name
    fn es_id(&self) -> Option<String>; // provides the elasticsearch id
}

pub trait Members {
    fn label(&self) -> &str;
    fn admins(&self) -> Vec<Arc<Admin>>;
}

impl<'a, T: MimirObject> MimirObject for &'a T {
    fn is_geo_data() -> bool {
        T::is_geo_data()
    }
    fn doc_type() -> &'static str {
        T::doc_type()
    }
    fn es_id(&self) -> Option<String> {
        T::es_id(self)
    }
}

impl<T: MimirObject> MimirObject for Rc<T> {
    fn is_geo_data() -> bool {
        T::is_geo_data()
    }
    fn doc_type() -> &'static str {
        T::doc_type()
    }
    fn es_id(&self) -> Option<String> {
        T::es_id(self)
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Property {
    pub key: String,
    pub value: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Poi {
    pub id: String,
    pub label: String,
    pub name: String,
    pub coord: Coord,
    pub administrative_regions: Vec<Arc<Admin>>,
    pub weight: f64,
    pub zip_codes: Vec<String>,
    pub poi_type: PoiType,
    pub properties: Vec<Property>,
    pub address: Option<Address>,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoiType {
    pub id: String,
    pub name: String,
}

impl MimirObject for Poi {
    fn is_geo_data() -> bool {
        true
    }
    fn doc_type() -> &'static str {
        "poi"
    }
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

impl Members for Poi {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        self.administrative_regions.clone()
    }
}

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
pub struct Code {
    pub name: String,
    pub value: String,
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct I18nProperties(pub Vec<Property>);

impl serde::Serialize for I18nProperties {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.0.iter().map(|p| (&p.key, &p.value)))
    }
}

impl<'de> Deserialize<'de> for I18nProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let properties = BTreeMap::<String, String>::deserialize(deserializer)?
            .into_iter()
            .collect();
        Ok(properties)
    }
}

impl FromIterator<(String, String)> for I18nProperties {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        let properties = iter
            .into_iter()
            .map(|(k, v)| Property {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect::<Vec<_>>();
        I18nProperties(properties)
    }
}

impl I18nProperties {
    pub fn get(&self, lang: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|p| p.key == lang)
            .map(|p| p.value.as_ref())
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stop {
    pub id: String,
    pub label: String,
    pub name: String,
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
}

impl MimirObject for Stop {
    fn is_geo_data() -> bool {
        false
    }
    fn doc_type() -> &'static str {
        "stop"
    }
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}
impl Members for Stop {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        self.administrative_regions.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Admin {
    pub id: String,
    pub insee: String,
    pub level: u32,
    pub label: String,
    pub name: String,
    pub zip_codes: Vec<String>,
    pub weight: f64,
    pub coord: Coord,
    #[serde(
        serialize_with = "custom_multi_polygon_serialize",
        deserialize_with = "custom_multi_polygon_deserialize",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub boundary: Option<geo::MultiPolygon<f64>>,

    #[serde(
        serialize_with = "serialize_bbox",
        deserialize_with = "deserialize_bbox",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub bbox: Option<geo::Bbox<f64>>,

    #[serde(default)]
    pub zone_type: Option<ZoneType>,
    #[serde(default)]
    pub parent_id: Option<String>, // id of the Admin's parent (from the cosmogony's hierarchy)

    #[serde(default)]
    pub codes: Vec<Code>,

    #[serde(default)]
    pub names: I18nProperties,

    #[serde(default)]
    pub labels: I18nProperties,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,
}

impl Admin {
    pub fn is_city(&self) -> bool {
        match self.zone_type {
            Some(ZoneType::City) => true,
            _ => false,
        }
    }
}

fn custom_multi_polygon_serialize<S>(
    multi_polygon_option: &Option<geo::MultiPolygon<f64>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use geojson::{GeoJson, Geometry, Value};
    use serde::Serialize;

    match *multi_polygon_option {
        Some(ref multi_polygon) => {
            GeoJson::Geometry(Geometry::new(Value::from(multi_polygon))).serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}

fn custom_multi_polygon_deserialize<'de, D>(
    d: D,
) -> Result<Option<geo::MultiPolygon<f64>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use geojson;
    use geojson::conversion::TryInto;
    use serde::Deserialize;

    Option::<geojson::GeoJson>::deserialize(d).map(|option| {
        option.and_then(|geojson| match geojson {
            geojson::GeoJson::Geometry(geojson_geom) => {
                let geo_geom: Result<geo::Geometry<f64>, _> = geojson_geom.value.try_into();
                match geo_geom {
                    Ok(geo::Geometry::MultiPolygon(geo_multi_polygon)) => Some(geo_multi_polygon),
                    Ok(_) => None,
                    Err(e) => {
                        warn!("Error deserializing geometry: {}", e);
                        None
                    }
                }
            }
            _ => None,
        })
    })
}

pub fn serialize_bbox<'a, S>(
    bbox: &'a Option<geo::Bbox<f64>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;

    match bbox {
        Some(b) => {
            // bbox serialized as an array
            // using GeoJSON bounding box format
            // See RFC 7946: https://tools.ietf.org/html/rfc7946#section-5
            let geojson_bbox: geojson::Bbox = vec![b.xmin, b.ymin, b.xmax, b.ymax];
            geojson_bbox.serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}

fn deserialize_bbox<'de, D>(d: D) -> Result<Option<geo::Bbox<f64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    Option::<Vec<f64>>::deserialize(d).map(|option| {
        option.map(|b| geo::Bbox {
            xmin: b[0],
            ymin: b[1],
            xmax: b[2],
            ymax: b[3],
        })
    })
}

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

impl MimirObject for Admin {
    fn is_geo_data() -> bool {
        true
    }
    fn doc_type() -> &'static str {
        "admin"
    }
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Street {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub administrative_regions: Vec<Arc<Admin>>,
    pub label: String,
    pub weight: f64,
    pub coord: Coord,
    pub zip_codes: Vec<String>,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,
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
        "street"
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Addr {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub house_number: String,
    pub street: Street,
    pub label: String,
    pub coord: Coord,
    pub weight: f64,
    pub zip_codes: Vec<String>,
    /// Distance to the coord in query.
    /// Not serialized as is because it is returned in the `Feature` object
    #[serde(default, skip)]
    pub distance: Option<u32>,
}

impl MimirObject for Addr {
    fn is_geo_data() -> bool {
        true
    }
    fn doc_type() -> &'static str {
        "addr"
    }
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

impl Members for Addr {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Arc<Admin>> {
        self.street.admins()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasOperations {
    pub actions: Vec<AliasOperation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<AliasParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<AliasParameter>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasParameter {
    pub index: String,
    pub alias: String,
}

// we want a custom serialization for coords, and so far the cleanest way
// to do this that has been found is to wrap the coord in another struct
#[derive(Debug, Clone)]
pub struct Coord(pub geo::Coordinate<f64>);
impl Coord {
    pub fn new(lon: f64, lat: f64) -> Coord {
        Coord(geo::Coordinate { x: lon, y: lat })
    }
    pub fn lon(&self) -> f64 {
        self.x
    }
    pub fn lat(&self) -> f64 {
        self.y
    }
    pub fn is_default(&self) -> bool {
        self.lat() == 0. && self.lon() == 0.
    }
    pub fn is_valid(&self) -> bool {
        !self.is_default()
            && -90. <= self.lat()
            && self.lat() <= 90.
            && -180. <= self.lon()
            && self.lon() <= 180.
    }
}
impl Default for Coord {
    fn default() -> Coord {
        Coord(geo::Coordinate { x: 0., y: 0. })
    }
}

impl ::std::ops::Deref for Coord {
    type Target = geo::Coordinate<f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for Coord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_struct("Coord", 2)?;
        ser.serialize_field("lon", &self.0.x)?;
        ser.serialize_field("lat", &self.0.y)?;
        ser.end()
    }
}

impl<'de> Deserialize<'de> for Coord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Lon,
            Lat,
        };

        struct CoordVisitor;

        impl<'de> Visitor<'de> for CoordVisitor {
            type Value = Coord;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct Coord")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Coord, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let lon = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let lat = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Coord::new(lon, lat))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Coord, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut lat = None;
                let mut lon = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Lat => {
                            if lat.is_some() {
                                return Err(de::Error::duplicate_field("lat"));
                            }
                            lat = Some(map.next_value()?);
                        }
                        Field::Lon => {
                            if lon.is_some() {
                                return Err(de::Error::duplicate_field("lon"));
                            }
                            lon = Some(map.next_value()?);
                        }
                    }
                }
                let lat = lat.ok_or_else(|| de::Error::missing_field("lat"))?;
                let lon = lon.ok_or_else(|| de::Error::missing_field("lon"))?;
                Ok(Coord::new(lon, lat))
            }
        }

        const FIELDS: &'static [&'static str] = &["lat", "lon"];
        deserializer.deserialize_struct("Coord", FIELDS, CoordVisitor)
    }
}
