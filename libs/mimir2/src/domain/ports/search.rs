/// This port defines a method to search
use crate::domain::model::query::Query;

// TODO: this is probably redundant with what is in Query
#[derive(Debug, Clone)]
pub struct SearchParameters {
    pub doc_types: Vec<String>,
    pub query: Query,
}
