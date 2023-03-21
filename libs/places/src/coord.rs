use geojson::Geometry;
use serde::{
    de::{self, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize,
};
use std::fmt;

// we want a custom serialization for coords, and so far the cleanest way
// to do this that has been found is to wrap the coord in another struct
#[derive(Debug, Clone, Copy)]
pub struct Coord(pub geo_types::Coord<f64>);

impl Coord {
    pub fn new(lon: f64, lat: f64) -> Coord {
        Coord(geo_types::Coord { x: lon, y: lat })
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
        Coord(geo_types::Coord { x: 0., y: 0. })
    }
}

impl ::std::ops::Deref for Coord {
    type Target = geo_types::Coord<f64>;
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

impl From<Coord> for Geometry {
    fn from(coord: Coord) -> Geometry {
        Geometry::new(geojson::Value::Point(vec![coord.lon(), coord.lat()]))
    }
}

impl From<Coord> for geo_types::Point<f64> {
    fn from(coord: Coord) -> geo_types::Point<f64> {
        geo_types::Point::new(coord.lon(), coord.lat())
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
        }

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

        const FIELDS: &[&str] = &["lat", "lon"];
        deserializer.deserialize_struct("Coord", FIELDS, CoordVisitor)
    }
}

impl From<&navitia_poi_model::Coord> for Coord {
    fn from(coord: &navitia_poi_model::Coord) -> Coord {
        Coord::new(coord.lon(), coord.lat())
    }
}
