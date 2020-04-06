

macro_rules! getter {
    ($settings:expr, $to_get:expr, $extra:ident) => {{
        let tmp = getter!($settings, $to_get)?;
        tmp.$extra().ok_or(format!(
            "Failed to call `{}` on `{}` entry from bragi-settings.json",
            stringify!($extra),
            $to_get,
        ))
    }};
    ($settings:expr, $to_get:expr) => {{
        $settings.pointer(&$to_get).ok_or(format!(
            "Missing `{}` entry from bragi-settings.json",
            $to_get
        ))
    }};
}

#[derive(Clone, Debug)]
pub struct TypeQueryBoosts {
    pub global: f64,
    pub address: f64,
    pub admin: f64,
    pub stop: f64,
    pub poi: f64,
    pub street: f64,
}

impl TypeQueryBoosts {
    fn new(settings: &serde_json::Value) -> Result<TypeQueryBoosts, String> {
        Ok(TypeQueryBoosts {
            global: getter!(settings, "/global_boost", as_f64)?,
            address: getter!(settings, "/boosts/address", as_f64)?,
            admin: getter!(settings, "/boosts/admin", as_f64)?,
            stop: getter!(settings, "/boosts/stop", as_f64)?,
            poi: getter!(settings, "/boosts/poi", as_f64)?,
            street: getter!(settings, "/boosts/street", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct StringQueryBoosts {
    pub global: f64,
    pub name: f64,
    pub label: f64,
    pub label_prefix: f64,
    pub zip_codes: f64,
    pub house_number: f64,
    pub label_ngram_with_coord: f64,
    pub label_ngram: f64,
}

impl StringQueryBoosts {
    fn new(settings: &serde_json::Value) -> Result<StringQueryBoosts, String> {
        Ok(StringQueryBoosts {
            global: getter!(settings, "/global_boost", as_f64)?,
            name: getter!(settings, "/boosts/name", as_f64)?,
            label: getter!(settings, "/boosts/label", as_f64)?,
            label_prefix: getter!(settings, "/boosts/label_prefix", as_f64)?,
            zip_codes: getter!(settings, "/boosts/zip_codes", as_f64)?,
            house_number: getter!(settings, "/boosts/house_number", as_f64)?,
            label_ngram_with_coord: getter!(settings, "/boosts/label_ngram_with_coord", as_f64)?,
            label_ngram: getter!(settings, "/boosts/label_ngram", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ImportanceQueryBoosts {
    pub admin_weight_fuzzy: f64,
    pub build_weight_fuzzy: f64,
    pub proximity_boost_fuzzy: f64,
    pub admin_weight: f64,
    pub build_weight: f64,
    pub proximity_boost: f64,
}

impl ImportanceQueryBoosts {
    fn new(settings: &serde_json::Value) -> Result<ImportanceQueryBoosts, String> {
        Ok(ImportanceQueryBoosts {
            admin_weight_fuzzy: getter!(settings, "/boosts/admin_weight_fuzzy", as_f64)?,
            build_weight_fuzzy: getter!(settings, "/boosts/build_weight_fuzzy", as_f64)?,
            proximity_boost_fuzzy: getter!(settings, "/boosts/proximity_boost_fuzzy", as_f64)?,
            admin_weight: getter!(settings, "/boosts/admin_weight", as_f64)?,
            build_weight: getter!(settings, "/boosts/build_weight", as_f64)?,
            proximity_boost: getter!(settings, "/boosts/proximity_boost", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct QuerySettings {
    pub type_query: TypeQueryBoosts,
    pub string_query: StringQueryBoosts,
    pub importance_query: ImportanceQueryBoosts,
}

impl QuerySettings {
    pub fn new(settings: &str) -> Result<QuerySettings, String> {
        let settings = serde_json::from_str::<serde_json::Value>(&settings)
            .map_err(|err| format!("Error occurred when reading bragi settings: {}", err))?;
        Ok(QuerySettings {
            type_query: TypeQueryBoosts::new(getter!(settings, "/type_query")?)?,
            string_query: StringQueryBoosts::new(getter!(settings, "/string_query")?)?,
            importance_query: ImportanceQueryBoosts::new(getter!(settings, "/string_query")?)?,
        })
    }
}