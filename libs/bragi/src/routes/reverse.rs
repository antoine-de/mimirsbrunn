use crate::extractors::BragiQuery;
use crate::routes::params;
use crate::{model, model::FromWithLang, Context};
use actix_web::{Json, State};
use mimir::rubber::Rubber;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    lat: f64,
    lon: f64,
    timeout: Option<Duration>,
}

pub fn reverse(
    params: BragiQuery<Params>,
    state: State<Context>,
) -> Result<Json<model::Autocomplete>, model::BragiError> {
    let timeout = params::get_timeout(&params.timeout, &state.max_es_timeout);
    let mut rubber = Rubber::new(&state.es_cnx_string);
    rubber.set_read_timeout(timeout);
    rubber.set_write_timeout(timeout);
    let coord = params::make_coord(params.lon, params.lat)?;
    rubber
        .get_address(&coord, timeout)
        .map_err(model::BragiError::from)
        .map(|r| model::Autocomplete::from_with_lang(r, None))
        .map(Json)
}
