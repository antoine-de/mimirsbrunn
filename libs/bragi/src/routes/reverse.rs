use crate::model::v1::AutocompleteResponse;
use crate::{model, Context};
use actix_web::{Json, Query, State};
use mimir::rubber::Rubber;
use std::time::Duration;
use mimir::objects::Coord;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    lat: f64,
    lon: f64,
    timeout: Option<Duration>,
}

pub fn reverse(
    params: Query<Params>,
    state: State<Context>,
) -> Result<Json<AutocompleteResponse>, model::BragiError> {
    let timeout = params.timeout; // TODO correct timeout handling
    let mut rubber = Rubber::new(&state.es_cnx_string);
    rubber.set_read_timeout(timeout);
    rubber.set_write_timeout(timeout);
    let coord = Coord::new(params.lon,params.lat);
    rubber
        .get_address(&coord, timeout)
        .map_err(model::BragiError::from)
        .map(AutocompleteResponse::from)
        .map(Json)
}
