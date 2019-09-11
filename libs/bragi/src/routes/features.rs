use crate::extractors::BragiQuery;
use crate::{model, model::FromWithLang, query, Context};
use actix_http::http::header::{CacheControl, CacheDirective};
use actix_web::web::{Data, HttpResponse, Path};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    #[serde(default)]
    pt_dataset: Vec<String>,
    #[serde(default)]
    poi_dataset: Vec<String>,
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    /// timeout in milliseconds
    timeout: Option<u64>,
}

pub fn features(
    params: BragiQuery<Params>,
    state: Data<Context>,
    id: Path<String>,
) -> Result<HttpResponse, model::BragiError> {
    let rubber = state.get_rubber_for_features(params.timeout.map(Duration::from_millis));
    let features = query::features(
        &params
            .pt_dataset
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        &params
            .poi_dataset
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        params.all_data,
        &*id,
        rubber,
    );
    features
        .map(|r| model::Autocomplete::from_with_lang(r, None))
        .map(|v| {
            HttpResponse::Ok()
                .set(CacheControl(vec![CacheDirective::MaxAge(
                    state.http_cache_duration,
                )]))
                .json(v)
        })
}
