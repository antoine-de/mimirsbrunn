#[derive(Debug, Clone)]
pub struct ListParameters {
    pub doc_type: String,
}

#[derive(Debug, Clone)]
pub struct SearchParameters {
    pub doc_types: Vec<String>,
    // A valid query DSL
    pub dsl: String,
}

#[derive(Debug, Clone)]
pub struct ExplainParameters {
    pub doc_type: String,
    // A valid query DSL
    pub dsl: String,
    pub id: String,
}
