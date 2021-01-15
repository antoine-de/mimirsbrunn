use crate::model::BragiError;
use mimir::objects::Coord;

pub fn make_coord(lon: f64, lat: f64) -> Result<Coord, BragiError> {
    if !(-90f64..=90f64).contains(&lat) {
        Err(BragiError::InvalidParam("lat is not a valid latitude"))
    } else if !(-180f64..=180f64).contains(&lon) {
        Err(BragiError::InvalidParam("lon is not a valid longitude"))
    } else {
        Ok(Coord::new(lon, lat))
    }
}
