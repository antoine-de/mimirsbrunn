use crate::extractors::BragiQuery;
use crate::routes::params;
use crate::{model, model::FromWithLang, Context};
use actix_web::{Json, State};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    lat: f64,
    lon: f64,
    /// timeout in milliseconds
    timeout: Option<u64>,
}

pub fn reverse(
    params: BragiQuery<Params>,
    state: State<Context>,
) -> Result<Json<model::Autocomplete>, model::BragiError> {
    // let mut rubber = state.get_rubber(params.timeout.map(Duration::from_millis));
    // let coord = params::make_coord(params.lon, params.lat)?;
    // rubber
    //     .get_address(&coord)
    //     .map_err(model::BragiError::from)
    //     .map(|r| model::Autocomplete::from_with_lang(r, None))
    //     .map(Json)
    unimplemented!()
}
