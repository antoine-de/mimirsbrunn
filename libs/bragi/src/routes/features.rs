use crate::extractors::BragiQuery;
use crate::{model, model::FromWithLang, query, Context};
use actix_web::{Json, Path, State};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    #[serde(default)]
    pt_dataset: Vec<String>,
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    /// timeout in milliseconds
    timeout: Option<u64>,
}

pub fn features(
    params: BragiQuery<Params>,
    state: State<Context>,
    id: Path<String>,
) -> Result<Json<model::Autocomplete>, model::BragiError> {
    let rubber = state.get_rubber(params.timeout.map(Duration::from_millis));
    let features = query::features(
        &params
            .pt_dataset
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        params.all_data,
        &*id,
        rubber,
    );
    features
        .map(|r| model::Autocomplete::from_with_lang(r, None))
        .map(Json)
}
