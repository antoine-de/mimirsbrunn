//! ES response for various ES queries, these only serialize the fields that we use,
/// which can be prone to change in the future
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// ES response for a search query.
#[derive(Deserialize)]
pub struct ElasticsearchSearchResponse<D> {
    pub pit_id: Option<String>,
    pub hits: ElasticsearchHits<D>,
}

#[derive(Deserialize)]
pub struct ElasticsearchHits<D> {
    pub hits: Vec<ElasticsearchHit<D>>,
}

#[derive(Deserialize)]
pub struct ElasticsearchHit<D> {
    #[serde(rename = "_source")]
    pub source: D,
    #[serde(default)]
    pub sort: Vec<Value>,
}

impl<D> ElasticsearchSearchResponse<D> {
    /// Consume the response into an iterator over the responded documents.
    pub fn into_hits(self) -> impl Iterator<Item = D> {
        self.hits.hits.into_iter().map(|hit| hit.source)
    }
}

/// ES response for a get query.
#[derive(Deserialize)]
pub struct ElasticsearchGetResponse<D> {
    pub docs: Vec<ElasticsearchDocs<D>>,
}

#[derive(Deserialize)]
pub struct ElasticsearchDocs<D> {
    #[serde(rename = "_source")]
    pub source: D,
    pub found: bool,
}

impl<D> ElasticsearchGetResponse<D> {
    /// Consume the response into an iterator over the responded documents.
    pub fn into_docs(self) -> impl Iterator<Item = D> {
        self.docs.into_iter().map(|doc| doc.source)
    }
}

/// ES response for bulk insert queries.
#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct ElasticsearchBulkResponse {
    pub items: Vec<ElasticsearchBulkItem>,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElasticsearchBulkItem {
    Index(ElasticsearchBulkStatus),
    Update(ElasticsearchBulkStatus),
}

impl ElasticsearchBulkItem {
    pub fn inner(self) -> ElasticsearchBulkStatus {
        match self {
            ElasticsearchBulkItem::Index(inner) => inner,
            ElasticsearchBulkItem::Update(inner) => inner,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct ElasticsearchBulkStatus {
    pub status: u16,
    #[serde(flatten, deserialize_with = "deserialize_bulk_result")]
    pub result: Result<ElasticsearchBulkResult, ElasticsearchBulkError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElasticsearchBulkResult {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct ElasticsearchBulkError {
    #[serde(rename = "type")]
    pub err_type: String,
    pub reason: String,
    pub index: Option<String>,
    pub index_uuid: Option<String>,
    pub shard: Option<String>,
}

// Custom deserializers

/// Serialize { "result": T, "error": E } into a Result<T, E>
pub fn deserialize_bulk_result<'de, D, T: Deserialize<'de>, E: Deserialize<'de>>(
    deserializer: D,
) -> Result<Result<T, E>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ElasticsearchResult<T, E> {
        Result(T),
        Error(E),
    }

    Ok({
        match Deserialize::deserialize(deserializer)? {
            ElasticsearchResult::Result(x) => Ok(x),
            ElasticsearchResult::Error(err) => Err(err),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a sample response for Bulk API adapted from documentation:
    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html#docs-bulk-api-example
    fn sample() -> serde_json::Value {
        json!({
            "took": 486,
            "errors": true,
            "items": [
                {
                    "index": {
                        "_index": "index1",
                        "_type" : "_doc",
                        "_id": "5",
                        "status": 404,
                        "error": {
                            "type": "document_missing_exception",
                            "reason": "[_doc][5]: document missing",
                            "index_uuid": "aAsFqTI0Tc2W0LCWgPNrOA",
                            "shard": "0",
                            "index": "index1"
                        }
                    }
                },
                {
                    "update": {
                        "_index": "index1",
                        "_type" : "_doc",
                        "_id": "7",
                        "_version": 1,
                        "result": "created",
                        "_shards": {
                            "total": 2,
                            "successful": 1,
                            "failed": 0
                        },
                        "_seq_no": 0,
                        "_primary_term": 1,
                        "status": 201
                    }
                }
            ]
        })
    }

    #[test]
    fn test_elasticsearch_bulk_response_model() {
        let response: ElasticsearchBulkResponse = serde_json::from_value(sample()).unwrap();

        assert_eq!(
            response,
            ElasticsearchBulkResponse {
                items: vec![
                    ElasticsearchBulkItem::Index(ElasticsearchBulkStatus {
                        status: 404,
                        result: Err(ElasticsearchBulkError {
                            err_type: "document_missing_exception".to_string(),
                            reason: "[_doc][5]: document missing".to_string(),
                            index: "index1".to_string().into(),
                            index_uuid: "aAsFqTI0Tc2W0LCWgPNrOA".to_string().into(),
                            shard: "0".to_string().into(),
                        })
                    }),
                    ElasticsearchBulkItem::Update(ElasticsearchBulkStatus {
                        status: 201,
                        result: Ok(ElasticsearchBulkResult::Created)
                    })
                ]
            }
        )
    }
}
