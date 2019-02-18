use crate::extractors::BragiQuery;
use crate::routes::params;
use crate::{model, model::FromWithLang, query, Context};
use actix_web::{Json, Path, State};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    #[serde(default)]
    pt_dataset: Vec<String>,
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    timeout: Option<Duration>,
}

pub fn features(
    params: BragiQuery<Params>,
    state: State<Context>,
    id: Path<String>,
) -> Result<Json<model::Autocomplete>, model::BragiError> {
    let timeout = params::get_timeout(&params.timeout, &state.max_es_timeout);
    let features = query::features(
        &params
            .pt_dataset
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        params.all_data,
        &state.es_cnx_string,
        &*id,
        timeout,
    );
    features
        .map(|r| model::Autocomplete::from_with_lang(r, None))
        .map(Json)
}
