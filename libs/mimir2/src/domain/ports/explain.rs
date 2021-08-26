use crate::domain::model::query::Query;
/// This port defines a method to debug queries / settings

// TODO: this is redundant with what is in query
#[derive(Debug, Clone)]
pub struct ExplainParameters {
    pub doc_type: String,
    pub query: Query,
    pub id: String,
}
