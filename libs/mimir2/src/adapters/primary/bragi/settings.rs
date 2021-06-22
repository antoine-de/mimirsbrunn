use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Types {
    pub address: f64,
    pub admin: f64,
    pub stop: f64,
    pub poi: f64,
    pub street: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TypeQueryBoosts {
    pub global: f64,
    pub boosts: Types,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StringQueryBoosts {
    pub name: f64,
    pub label: f64,
    pub label_prefix: f64,
    pub zip_codes: f64,
    pub house_number: f64,
    pub label_ngram_with_coord: f64,
    pub label_ngram: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StringQuery {
    pub global: f64,
    pub boosts: StringQueryBoosts,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Proximity {
    pub weight: f64,
    pub weight_fuzzy: f64,
    pub decay: Decay,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Decay {
    pub func: String,
    pub scale: f64,
    pub offset: f64,
    pub decay: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BuildWeight {
    pub admin: f64,
    pub factor: f64,
    pub missing: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Weights {
    pub radius_range: (f64, f64),
    pub max_radius: BuildWeight,
    pub min_radius_prefix: BuildWeight,
    pub min_radius_fuzzy: BuildWeight,
    #[serde(flatten)]
    pub types: Types,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ImportanceQueryBoosts {
    pub proximity: Proximity,
    pub weights: Weights,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QuerySettings {
    pub type_query: TypeQueryBoosts,
    pub string_query: StringQuery,
    pub importance_query: ImportanceQueryBoosts,
}

impl QuerySettings {
    pub fn new(settings: &str) -> Result<QuerySettings, String> {
        toml::from_str(settings).map_err(|e| e.to_string())
    }
}
