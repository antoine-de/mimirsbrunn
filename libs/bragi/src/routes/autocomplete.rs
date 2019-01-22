use crate::{model, query, Context};
use actix_web::{Json, Query, Result, State};
use derivative::Derivative;
use navitia_model::objects::Coord;
use std::time::Duration;

// TODO: pretty errors, async es

// TODO use params::Type
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
enum Type {
    #[serde(rename = "city")]
    City,
    #[serde(rename = "house")]
    House,
    #[serde(rename = "poi")]
    Poi,
    #[serde(rename = "public_transport:stop_area")]
    StopArea,
    #[serde(rename = "street")]
    Street,
}

impl Type {
    fn as_str(&self) -> &'static str {
        match self {
            &Type::City => "city",
            &Type::House => "house",
            &Type::Poi => "poi",
            &Type::StopArea => "public_transport:stop_area",
            &Type::Street => "street",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
#[derivative(Default)]
pub struct Params {
    q: String,
    #[serde(rename = "pt_dataset[]", default)]
    pt_datasets: Vec<String>, // TODO make the multiple params work
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    //Note: for the moment we can't use an external struct and flatten it (https://github.com/nox/serde_urlencoded/issues/33)
    #[serde(default)]
    #[derivative(Default(value = "10u64"))]
    limit: u64,
    #[serde(default)]
    offset: u64,
    timeout: Option<Duration>, //TODO custom default timeout
    coord: Option<Coord>,
    #[serde(rename = "types[]", default)]
    types: Vec<Type>, // TODO make the multiple params work
}

impl Params {
    fn types_as_str(&self) -> Vec<&str> {
        self.types.iter().map(Type::as_str).collect()
    }
}

pub fn autocomplete(
    params: Query<Params>,
    state: State<Context>,
) -> Result<Json<model::v1::AutocompleteResponse>> {
    println!("{:?}", *params);
    let res = query::autocomplete(
        &params.q,
        &params
            .pt_datasets
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        params.all_data,
        params.offset,
        params.limit,
        params.coord,
        &state.es_cnx_string,
        None,
        &params.types_as_str(),
        params.timeout,
    );
    Ok(Json(res.into()))
}
