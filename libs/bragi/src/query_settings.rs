macro_rules! getter {
    ($settings:expr, $to_get:expr, $extra:ident) => {{
        let tmp = getter!($settings, $to_get)?;
        tmp.$extra().ok_or(format!(
            "Failed to call `{}` on `{}` entry",
            stringify!($extra),
            $to_get,
        ))
    }};
    ($settings:expr, $to_get:expr) => {{
        $settings
            .pointer(&$to_get)
            .ok_or(format!("Missing `{}` entry", $to_get))
    }};
}

#[derive(Clone, Debug)]
pub struct Types {
    pub address: f64,
    pub admin: f64,
    pub stop: f64,
    pub poi: f64,
    pub street: f64,
}

impl Types {
    fn new(settings: &serde_json::Value) -> Result<Types, String> {
        Ok(Types {
            address: getter!(settings, "/address", as_f64)?,
            admin: getter!(settings, "/admin", as_f64)?,
            stop: getter!(settings, "/stop", as_f64)?,
            poi: getter!(settings, "/poi", as_f64)?,
            street: getter!(settings, "/street", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TypeQueryBoosts {
    pub global: f64,
    pub types: Types,
}

impl TypeQueryBoosts {
    fn new(settings: &serde_json::Value) -> Result<TypeQueryBoosts, String> {
        Ok(TypeQueryBoosts {
            global: getter!(settings, "/global_boost", as_f64)?,
            types: Types::new(getter!(settings, "/boosts")?)?,
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
            label_prefix: getter!(settings, "/boosts/label.prefix", as_f64)?,
            zip_codes: getter!(settings, "/boosts/zip_codes", as_f64)?,
            house_number: getter!(settings, "/boosts/house_number", as_f64)?,
            label_ngram_with_coord: getter!(settings, "/boosts/label.ngram_with_coord", as_f64)?,
            label_ngram: getter!(settings, "/boosts/label.ngram", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Proximity {
    pub weight: f64,
    pub weight_fuzzy: f64,
    pub decay_distance: f64,
    pub offset_distance: f64,
    pub decay: f64,
}

impl Proximity {
    fn new(settings: &serde_json::Value) -> Result<Proximity, String> {
        Ok(Proximity {
            weight: getter!(settings, "/weight", as_f64)?,
            weight_fuzzy: getter!(settings, "/weight_fuzzy", as_f64)?,
            decay_distance: getter!(settings, "/decay_distance", as_f64)?,
            offset_distance: getter!(settings, "/offset_distance", as_f64)?,
            decay: getter!(settings, "/decay", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BuildWeight {
    pub admin: f64,
    pub factor: f64,
    pub missing: f64,
}

impl BuildWeight {
    fn new(settings: &serde_json::Value) -> Result<BuildWeight, String> {
        Ok(BuildWeight {
            admin: getter!(settings, "/admin", as_f64)?,
            factor: getter!(settings, "/factor", as_f64)?,
            missing: getter!(settings, "/missing", as_f64)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Weights {
    pub coords_fuzzy: BuildWeight,
    pub coords: BuildWeight,
    pub no_coords: BuildWeight,
    pub types: Types,
}

impl Weights {
    fn new(settings: &serde_json::Value) -> Result<Weights, String> {
        Ok(Weights {
            coords_fuzzy: BuildWeight::new(getter!(settings, "/coords_fuzzy")?)?,
            coords: BuildWeight::new(getter!(settings, "/coords")?)?,
            no_coords: BuildWeight::new(getter!(settings, "/no_coords")?)?,
            types: Types::new(settings)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ImportanceQueryBoosts {
    pub proximity: Proximity,
    pub weights: Weights,
}

impl ImportanceQueryBoosts {
    fn new(settings: &serde_json::Value) -> Result<ImportanceQueryBoosts, String> {
        Ok(ImportanceQueryBoosts {
            proximity: Proximity::new(getter!(settings, "/proximity")?)?,
            weights: Weights::new(getter!(settings, "/weights")?)?,
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
            importance_query: ImportanceQueryBoosts::new(getter!(settings, "/importance_query")?)?,
        })
    }
}
