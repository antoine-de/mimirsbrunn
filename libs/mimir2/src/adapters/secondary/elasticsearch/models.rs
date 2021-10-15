use serde::Deserialize;
use serde_json::Value;

/// ES response for a search query, this only serialize the fields that we use,
/// which can be prone to change in the future.
#[derive(Deserialize)]
pub struct EsResponse<D> {
    pub pit_id: Option<String>,
    pub hits: EsHits<D>,
}

#[derive(Deserialize)]
pub struct EsHits<D> {
    pub hits: Vec<EsHit<D>>,
}

#[derive(Deserialize)]
pub struct EsHit<D> {
    #[serde(rename = "_source")]
    pub source: D,
    #[serde(default)]
    pub sort: Vec<Value>,
}

impl<D> EsResponse<D> {
    /// Consume the response into an iterator over the responded documents.
    pub fn into_hits(self) -> impl Iterator<Item = D> {
        self.hits.hits.into_iter().map(|hit| hit.source)
    }
}
