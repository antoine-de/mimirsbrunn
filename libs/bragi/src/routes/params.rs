use crate::model::BragiError;
use mimir::objects::Coord;
use std::time::Duration;

pub fn get_timeout(
    query_timeout: &Option<Duration>,
    default_timeout: &Option<Duration>,
) -> Option<Duration> {
    query_timeout
        .clone()
        .map(|t| match default_timeout {
            Some(dt) => t.min(*dt),
            None => t,
        })
        .or_else(|| default_timeout.clone())
}

pub fn make_coord(lon: f64, lat: f64) -> Result<Coord, BragiError> {
    if lon < -90f64 || lon > 90f64 {
        Err(BragiError::InvalidParam("lon is not a valid longitude"))
    } else if lat < -180f64 || lat > 180f64 {
        Err(BragiError::InvalidParam("lat is not a valid latitude"))
    } else {
        Ok(Coord::new(lon, lat))
    }
}
