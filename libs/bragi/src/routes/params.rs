use crate::model::BragiError;
use mimir::objects::Coord;

pub fn make_coord(lon: f64, lat: f64) -> Result<Coord, BragiError> {
    if lon < -90f64 || lon > 90f64 {
        Err(BragiError::InvalidParam("lon is not a valid longitude"))
    } else if lat < -180f64 || lat > 180f64 {
        Err(BragiError::InvalidParam("lat is not a valid latitude"))
    } else {
        Ok(Coord::new(lon, lat))
    }
}
