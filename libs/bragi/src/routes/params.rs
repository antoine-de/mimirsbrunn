use crate::model::BragiError;
use mimir::objects::Coord;

pub fn make_coord(lon: f64, lat: f64) -> Result<Coord, BragiError> {
    if lat < -90f64 || lat > 90f64 {
        Err(BragiError::InvalidParam("lat is not a valid latitude"))
    } else if lon < -180f64 || lon > 180f64 {
        Err(BragiError::InvalidParam("lon is not a valid longitude"))
    } else {
        Ok(Coord::new(lon, lat))
    }
}
