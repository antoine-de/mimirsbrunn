#[derive(Debug, Clone)]
pub struct QueryParameters {
    pub containers: Vec<String>, // if you want to target all indices, use vec![munin]
    pub dsl: String,             // if you want to target all documents, use { match_all: {} }
}
