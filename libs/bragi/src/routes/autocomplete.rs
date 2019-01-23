use crate::model::v1::AutocompleteResponse;
use crate::model::BragiError;
use crate::{model, query, Context};
use actix_web::{Json, Query, State};
use geojson::GeoJson;
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

fn default_limit() -> u64 {
    10u64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Params {
    q: String,
    #[serde(rename = "pt_dataset[]", default)]
    pt_datasets: Vec<String>, // TODO make the multiple params work
    #[serde(rename = "_all_data", default)]
    all_data: bool,
    //Note: for the moment we can't use an external struct and flatten it (https://github.com/nox/serde_urlencoded/issues/33)
    #[serde(default = "default_limit")]
    limit: u64,
    #[serde(default)]
    offset: u64,
    timeout: Option<Duration>, //TODO custom default timeout
    lat: Option<f64>,
    lon: Option<f64>,
    #[serde(rename = "types[]", default)]
    types: Vec<Type>, // TODO make the multiple params work
}

impl Params {
    fn types_as_str(&self) -> Vec<&str> {
        self.types.iter().map(Type::as_str).collect()
    }
    fn coord(&self) -> Option<Coord> {
        match (self.lon, self.lat) {
            (Some(lon), Some(lat)) => Some(Coord { lon, lat }),
            _ => None,
        }
    }
}

pub fn call_autocomplete(
    params: &Params,
    state: &Context,
    shape: Option<Vec<(f64, f64)>>,
) -> Result<Json<AutocompleteResponse>, model::BragiError> {
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
        params.coord(),
        &state.es_cnx_string,
        shape,
        &params.types_as_str(),
        params.timeout,
    );
    res.map(AutocompleteResponse::from).map(Json)
}

pub fn autocomplete(
    params: Query<Params>,
    state: State<Context>,
) -> Result<Json<AutocompleteResponse>, model::BragiError> {
    println!("{:?}", *params);
    call_autocomplete(&*params, &*state, None)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonParams {
    shape: GeoJson,
}

impl JsonParams {
    fn get_es_shape(&self) -> Result<Vec<(f64, f64)>, model::BragiError> {
        match &self.shape {
            GeoJson::Feature(f) => {
                match &f.geometry {
                    Some(geom) => {
                        match &geom.value {
                            geojson::Value::Polygon(p) => {
                                match p.as_slice() {
                                    [p] => {
                                        dbg!(Ok(p
                                            .iter()
                                            .filter_map(|c: &Vec<f64>| c.get(0..=1))
                                            .map(|c| (c[1], c[0])) // Note: the coord are inverted for ES
                                            .collect()))
                                    }
                                    _ => Err(BragiError::InvalidShape(
                                        "only polygon without holes are supported",
                                    )), //only polygon without holes are supported by elasticsearch
                                }
                            }
                            _ => Err(BragiError::InvalidShape("only polygon are supported")),
                        }
                    }
                    None => Err(BragiError::InvalidShape("no geometry")),
                }
            }
            _ => Err(BragiError::InvalidShape("only 'feature' is supported")),
        }
    }
}

pub fn post_autocomplete(
    params: Query<Params>,
    state: State<Context>,
    json_params: Json<JsonParams>,
) -> Result<Json<AutocompleteResponse>, model::BragiError> {
    println!(
        "POST autocomplete {:?} -------- {:?}",
        *params, *json_params
    );
    call_autocomplete(&*params, &*state, Some(json_params.get_es_shape()?))
}
