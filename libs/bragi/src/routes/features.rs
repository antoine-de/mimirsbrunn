use crate::model::v1::AutocompleteResponse;
use crate::{model, query, Context};
use actix_web::{Json, Path, Query, State};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    #[serde(rename = "pt_dataset[]", default)]
    pt_datasets: Vec<String>, // TODO make the multiple params work
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    timeout: Option<Duration>,
}

pub fn features(
    params: Query<Params>,
    state: State<Context>,
    id: Path<String>,
) -> Result<Json<AutocompleteResponse>, model::BragiError> {
    let timeout = params.timeout; // TODO correct timeout handling
    let features = query::features(
        &params
            .pt_datasets
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        params.all_data,
        &state.es_cnx_string,
        &*id,
        timeout,
    );
    Ok(Json(model::v1::AutocompleteResponse::from(features)))
}
