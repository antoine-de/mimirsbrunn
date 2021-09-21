use elasticsearch::cat::CatIndicesParts;
use elasticsearch::cluster::ClusterHealthParts;
use elasticsearch::http::response::Exception;
use elasticsearch::indices::{
    IndicesCreateParts, IndicesDeleteAliasParts, IndicesDeleteParts, IndicesGetAliasParts,
    IndicesPutAliasParts, IndicesRefreshParts,
};
use elasticsearch::ingest::IngestPutPipelineParts;
use elasticsearch::{
    BulkOperation, BulkParts, Elasticsearch, ExplainParts, OpenPointInTimeParts, SearchParts,
};
use futures::stream::{self, Stream, StreamExt};
use lazy_static::lazy_static;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use snafu::{ResultExt, Snafu};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use super::configuration::IndexConfiguration;
use super::ElasticsearchStorage;
use crate::domain::model::{
    configuration,
    index::{Index, IndexStatus},
    query::Query,
    stats::InsertStats as ModelInsertStats,
    status::{StorageHealth, StorageVersion},
};
use common::document::Document;

static CHUNK_SIZE: usize = 100;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid Elasticsearch Index Configuration: {} [{}]", source, details))]
    InvalidConfiguration {
        details: String,
        source: config::ConfigError,
    },

    /// Elasticsearch Error
    #[snafu(display("Elasticsearch Error: {} [{}]", source, details))]
    ElasticsearchClient {
        details: String,
        source: elasticsearch::Error,
    },

    /// Elasticsearch Not Created
    #[snafu(display("Elasticsearch Response: Not Created: {}", details))]
    NotCreated { details: String },

    /// Elasticsearch Not Deleted
    #[snafu(display("Elasticsearch Response: Not Deleted: {}", details))]
    NotDeleted { details: String },

    /// Elasticsearch Not Acknowledged
    #[snafu(display("Elasticsearch Response: Not Acknowledged: {}", details))]
    NotAcknowledged { details: String },

    /// Elasticsearch Document Insertion Exception
    #[snafu(display("Elasticsearch Failure without Exception: {}", details))]
    ElasticsearchFailureWithoutException { details: String },

    /// Elasticsearch Unhandled Exception
    #[snafu(display("Elasticsearch Unhandled Exception: {}", details))]
    ElasticsearchUnhandledException { details: String },

    /// Elasticsearch Duplicate Index
    #[snafu(display("Elasticsearch Duplicate Index: {}", index))]
    ElasticsearchDuplicateIndex { index: String },

    /// Elasticsearch Failed To Parse
    #[snafu(display("Elasticsearch Failed to Parse"))]
    ElasticsearchFailedToParse,

    /// Elasticsearch Failed To Parse
    #[snafu(display("Elasticsearch Failed to Parse Mapping of {}: {}", object, reason))]
    ElasticsearchInvalidMapping { object: String, reason: String },

    /// Elasticsearch Unknown Index
    #[snafu(display("Elasticsearch Unknown Index: {}", index))]
    ElasticsearchUnknownIndex { index: String },

    /// Elasticsearch Unknown Setting
    #[snafu(display("Elasticsearch Unknown Setting: {}", setting))]
    ElasticsearchUnknownSetting { setting: String },

    /// Elasticsearch Failed To Parse
    #[snafu(display("Elasticsearch Failed to Parse Index Settings: {}", reason))]
    ElasticsearchInvalidIndexSettings { reason: String },

    /// Elasticsearch Index Conversion
    #[snafu(display("Index Conversion Error: {}", details))]
    IndexConversion { details: String },

    /// Elasticsearch Deserialization Error
    #[snafu(display("JSON Elasticsearch Deserialization Error: {}", source))]
    ElasticsearchDeserialization { source: elasticsearch::Error },

    /// Elasticsearch Deserialization Error
    #[snafu(display("JSON Serde Deserialization Error: {}", source))]
    JsonDeserialization {
        source: serde_json::Error,
        details: String,
    },

    /// Invalid JSON Value
    #[snafu(display("JSON Deserialization Invalid: {} {:?}", details, json))]
    JsonInvalid { details: String, json: Value },

    /// Internal Error
    #[snafu(display("Internal Error: {}", reason))]
    Internal { reason: String },

    /// Elasticsearch Unhandled Status
    #[snafu(display("Elasticsearch Unhandled Status: {}", details))]
    ElasticsearchUnhandledStatus { details: String },
}

impl From<Exception> for Error {
    // This function analyzes the content of an elasticsearch exception,
    // and returns an error, the type of which should mirror the exception's content.
    // There is no clear blueprint for this analysis, it's very much adhoc.
    fn from(exception: Exception) -> Error {
        let root_cause = exception.error().root_cause();
        if root_cause.is_empty() {
            // If there is no root cause, there maybe a reason
            if let Some(reason) = exception.error().reason() {
                Error::ElasticsearchUnhandledException {
                    details: String::from(reason),
                }
            } else {
                Error::ElasticsearchUnhandledException {
                    details: String::from("Unspecified root cause or reason"),
                }
            }
        } else {
            lazy_static! {
                static ref ALREADY_EXISTS: Regex =
                    Regex::new(r"index \[([^\]/]+).*\] already exists").unwrap();
            }
            lazy_static! {
                static ref NOT_FOUND: Regex = Regex::new(r"no such index \[([^\]/]+).*\]").unwrap();
            }
            lazy_static! {
                static ref FAILED_PARSE: Regex = Regex::new(r"failed to parse").unwrap();
            }
            lazy_static! {
                // Example: Failed to parse mapping [_doc]: analyzer [ngram] has not been configured in mappings
                // we extract an 'object', between [], and the reason, behind ':'
                static ref FAILED_PARSE_MAPPING: Regex =
                    Regex::new(r"Failed to parse mapping \[([^\]/]+).*\]: (.*)").unwrap();
            }
            lazy_static! {
                static ref UNKNOWN_SETTING: Regex =
                    Regex::new(r"unknown setting \[([^\]/]+).*\]").unwrap();
            }
            match root_cause[0].reason() {
                Some(reason) => {
                    if let Some(caps) = ALREADY_EXISTS.captures(reason) {
                        let index = String::from(caps.get(1).unwrap().as_str());
                        Error::ElasticsearchDuplicateIndex { index }
                    } else if let Some(caps) = NOT_FOUND.captures(reason) {
                        let index = String::from(caps.get(1).unwrap().as_str());
                        Error::ElasticsearchUnknownIndex { index }
                    } else if let Some(caps) = FAILED_PARSE_MAPPING.captures(reason) {
                        let object = String::from(caps.get(1).unwrap().as_str());
                        let reason = String::from(caps.get(2).unwrap().as_str());
                        Error::ElasticsearchInvalidMapping { object, reason }
                    } else if FAILED_PARSE.is_match(reason) {
                        Error::ElasticsearchFailedToParse
                    } else if let Some(caps) = UNKNOWN_SETTING.captures(reason) {
                        let setting = String::from(caps.get(1).unwrap().as_str());
                        Error::ElasticsearchUnknownSetting { setting }
                    } else {
                        Error::ElasticsearchUnhandledException {
                            details: format!("Unidentified reason: {}", reason),
                        }
                    }
                }
                None => Error::ElasticsearchUnhandledException {
                    details: String::from("Unspecified reason"),
                },
            }
        }
    }
}

impl ElasticsearchStorage {
    pub fn new(client: Elasticsearch) -> ElasticsearchStorage {
        ElasticsearchStorage(client)
    }

    pub(super) async fn create_index(&self, config: IndexConfiguration) -> Result<(), Error> {
        let body = json!({ "mappings": config.mappings, "settings": config.settings });

        let response = self
            .0
            .indices()
            .create(IndicesCreateParts::Index(&config.name))
            .timeout(&config.parameters.timeout)
            .wait_for_active_shards(&config.parameters.wait_for_active_shards)
            .body(body)
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot create index '{}'", config.name),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"acknowledged": Bool(true), "index": String("name"), "shards_acknowledged": Bool(true)})
            // We verify that acknowledge is true, then add the cat indices API to get the full index.
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let acknowledged = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("acknowledged")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'acknowledged'"),
                    json: json.clone(),
                })?
                .as_bool()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON bool"),
                    json: json.clone(),
                })?;
            if acknowledged {
                Ok(())
            } else {
                Err(Error::NotCreated {
                    details: format!("index creation {}", config.name),
                })
            }
        } else {
            let exception = response.exception().await.ok().unwrap();
            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn delete_index(&self, index: String) -> Result<(), Error> {
        let response = self
            .0
            .indices()
            .delete(IndicesDeleteParts::Index(&[&index]))
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot find index '{}'", index),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"acknowledged": Bool(true), "index": String("name"), "shards_acknowledged": Bool(true)})
            // We verify that acknowledge is true, then add the cat indices API to get the full index.
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let acknowledged = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("acknowledged")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'acknowledged'"),
                    json: json.clone(),
                })?
                .as_bool()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON bool"),
                    json: json.clone(),
                })?;

            if acknowledged {
                Ok(())
            } else {
                Err(Error::NotDeleted {
                    details: String::from(
                        "Elasticsearch response to index deletion not acknowledged",
                    ),
                })
            }
        } else {
            let exception = response.exception().await.ok().unwrap();
            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    // FIXME Move details to impl ElasticsearchStorage.
    pub(super) async fn find_index(&self, index: String) -> Result<Option<Index>, Error> {
        let response = self
            .0
            .cat()
            .indices(CatIndicesParts::Index(&[&index]))
            .format("json")
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot find index '{}'", index),
            })?;

        if response.status_code().is_success() {
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let mut indices: Vec<ElasticsearchIndex> =
                serde_json::from_value(json).context(JsonDeserialization {
                    details: String::from("could not deserialize Elasticsearch indices"),
                })?;

            indices.pop().map(Index::try_from).transpose()
        } else {
            let exception = response.exception().await.ok().unwrap();

            // We need to handle this exception carefully, so that the 'unknown index' does
            // not result in an Error, but rather a Ok(None) to indicate that nothing was found.

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    if std::matches!(err, Error::ElasticsearchUnknownIndex { .. }) {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    /* Commented out because it is not used
           pub(super) async fn insert_document<D>(
           &self,
           index: String,
           id: String,
           document: D,
           ) -> Result<(), Error>
           where
           D: Serialize + Send + Sync + 'static,
           {
           let response = self
           .0
           .index(IndexParts::IndexId(&index, &id))
           .body(document)
           .send()
           .await
           .context(ElasticsearchClient {
           details: format!("cannot index document '{}'", id),
           })?;

           if response.status_code().is_success() {
        // Response similar to:
        // {
        //   "_id": "AvypLXkBazLmtM_qtw9a",
        //   "_index": "munin_book_books_20210502_151927_673737330",
        //   "_primary_term": 1, "_seq_no": 0,
        //   "_shards": {
        //     "failed": 0, "successful": 1, "total": 2
        //   },
        //   "_type": "_doc",
        //   "_version": 1,
        //   "result": "created"
        // }
        // We verify that result is "created"
        let json = response
        .json::<Value>()
        .await
        .context(ElasticsearchDeserialization)?;

        let result = json
        .as_object()
        .ok_or(Error::JsonInvalid {
        details: String::from("expected JSON object"),
        })?
        .get("result")
        .ok_or(Error::JsonInvalid {
        details: String::from("expected 'result'"),
        })?
        .as_str()
        .ok_or(Error::JsonInvalid {
        details: String::from("expected JSON string"),
        })?;
        if result == "created" {
        Ok(())
        } else {
        Err(Error::NotCreated {
        details: format!("document creation {}", id),
        })
        }
        } else {
        let exception = response.exception().await.ok().unwrap();

        match exception {
        Some(exception) => {
        let err = Error::from(exception);
        Err(err)
        }
        None => Err(Error::ElasticsearchFailureWithoutException {
        details: String::from("Fail status without exception"),
        }),
        }
    }
    }
    */

    // Changed the name to avoid recursive calls int storage::insert_documents
    pub(super) async fn insert_documents_in_index<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, Error>
    where
        //D: Document + Send + Sync + 'static,
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        let stats = Arc::new(Mutex::new(InsertStats::default()));

        documents
            .chunks(CHUNK_SIZE) // FIXME chunck size should be a variable.
            .for_each_concurrent(8, |chunk| {
                // FIXME 8 automagick!!??
                let stats = stats.clone();
                let index = index.clone();
                async move {
                    if let Err(err) = self.insert_chunk_in_index(index, chunk, stats).await {
                        panic!("Error inserting chunk: {}", err.to_string());
                    }
                } // res
            })
            .await;

        let lock = Arc::try_unwrap(stats).map_err(|_err| Error::Internal {
            reason: String::from("Lock has still multiple owners"),
        })?;

        let res = lock.into_inner().map_err(|_err| Error::Internal {
            reason: String::from("Mutex cannot be unlocked"),
        })?;

        Ok(res)
    }

    // Changed the name to avoid recursive calls int storage::insert_documents
    pub(super) async fn insert_chunk_in_index<D>(
        &self,
        index: String,
        chunk: Vec<D>,
        stats: Arc<Mutex<InsertStats>>,
    ) -> Result<(), Error>
    where
        D: Document + Send + Sync + 'static,
    {
        // We try to insert the chunk using bulk insertion.
        // We then analyze the result, which contains an array of 'items'.
        // Each item must contain the string 'created'. So we iterate through these items,
        // and build a Result<(), Error>, and skip while it `is_ok()`. If we have an
        // error, we report it.
        let mut ops: Vec<BulkOperation<Value>> = Vec::with_capacity(CHUNK_SIZE);
        chunk.into_iter().for_each(|doc| {
            let doc_id = doc.id();
            let value = serde_json::to_value(doc).expect("to json value");
            ops.push(BulkOperation::index(value).id(doc_id).into());
        });
        // FIXME Missing Error Handling
        let resp = self
            .0
            .bulk(BulkParts::Index(index.as_str()))
            .body(ops)
            .send()
            .await
            .expect("send bulk");

        if resp.status_code().is_success() {
            let json = resp
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let items = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("items")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'items'"),
                    json: json.clone(),
                })?
                .as_array()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'items' to be an array"),
                    json: json.clone(),
                })?;

            let mut res = items
                .iter()
                .map(|item| {
                    let result = item
                        .as_object()
                        .ok_or(Error::JsonInvalid {
                            details: String::from("expected JSON object"),
                            json: item.clone(),
                        })?
                        .get("index")
                        .ok_or(Error::JsonInvalid {
                            details: String::from("expected 'index'"),
                            json: item.clone(),
                        })?
                        .as_object()
                        .ok_or(Error::JsonInvalid {
                            details: String::from("expected JSON 'index' to be an object"),
                            json: item.clone(),
                        })?
                        .get("result")
                        .ok_or(Error::JsonInvalid {
                            details: String::from("expected 'result'"),
                            json: item.clone(),
                        })?
                        .as_str()
                        .ok_or(Error::JsonInvalid {
                            details: String::from("expected 'result' to be a string"),
                            json: item.clone(),
                        })?;

                    if result == "created" {
                        // println!("TRACE: item {:?} was created", item);
                        (*stats).lock().unwrap().created += 1;
                        // let mut stats_guard = (*stats).lock().unwrap().created += 1;
                        // *stats_guard.created += 1;
                        Ok(())
                    } else if result == "updated" {
                        (*stats).lock().unwrap().updated += 1;
                        warn!("Item was updated: {:?}", item);
                        Ok(())
                    } else {
                        Err(Error::NotCreated {
                            details: format!("not created: {:?}", item),
                        })
                    }
                })
                .skip_while(|x| x.is_ok());

            match res.next() {
                None => Ok(()),
                Some(err) => Err(err.unwrap_err()),
            }
        } else {
            let exception = resp.exception().await.ok().unwrap();
            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn add_alias(&self, indices: Vec<String>, alias: String) -> Result<(), Error> {
        let indices = indices.iter().map(String::as_str).collect::<Vec<_>>();
        let response = self
            .0
            .indices()
            .put_alias(IndicesPutAliasParts::IndexName(&indices, &alias))
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!(
                    "cannot add alias '{}' to indices '{}'",
                    alias,
                    indices.join(" ")
                ),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"acknowledged": Bool(true)})
            // We verify that acknowledge is true, then add the cat indices API to get the full index.
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let acknowledged = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("acknowledged")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'acknowledged'"),
                    json: json.clone(),
                })?
                .as_bool()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON boolean"),
                    json: json.clone(),
                })?;

            if acknowledged {
                Ok(())
            } else {
                Err(Error::NotAcknowledged {
                    details: format!("alias {} creation", alias),
                })
            }
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn remove_alias(
        &self,
        indices: Vec<String>,
        alias: String,
    ) -> Result<(), Error> {
        let indices = indices.iter().map(String::as_str).collect::<Vec<_>>();
        let response = self
            .0
            .indices()
            .delete_alias(IndicesDeleteAliasParts::IndexName(&indices, &[&alias]))
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!(
                    "cannot remove alias '{}' to indices '{}'",
                    alias,
                    indices.join(" ")
                ),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"acknowledged": Bool(true)})
            // We verify that acknowledge is true, then add the cat indices API to get the full index.
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let acknowledged = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("acknowledged")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'acknowledged'"),
                    json: json.clone(),
                })?
                .as_bool()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON boolean"),
                    json: json.clone(),
                })?;

            if acknowledged {
                Ok(())
            } else {
                Err(Error::NotAcknowledged {
                    details: format!("alias {} deletion", alias),
                })
            }
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn find_aliases(
        &self,
        index: String,
    ) -> Result<BTreeMap<String, Vec<String>>, Error> {
        // The last piece of the input index should be a dataset
        // If you didn't add the trailing '_*' below, when you would search for
        // the aliases of eg 'fr', you would also find the aliases for 'fr-ne'.
        let index = format!("{}_*", index);
        let response = self
            .0
            .indices()
            .get_alias(IndicesGetAliasParts::Index(&[&index]))
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot find aliases to {}", index),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // {
            //   "index1": {
            //      "aliases": {
            //         "alias1": {},
            //         "alias2": {}
            //      }
            //   },
            //   "index2": {
            //      "aliases": {
            //         "alias3": {}
            //      }
            //   }
            // }
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let aliases = json
                .as_object()
                .map(|indices| {
                    indices
                        .iter()
                        .filter_map(|(index, value)| {
                            value["aliases"]
                                .as_object()
                                .map(|aliases| (index.clone(), aliases.keys().cloned().collect()))
                        })
                        .collect()
                })
                .unwrap_or_else(|| {
                    info!("No alias for index {}", index);
                    BTreeMap::new()
                });
            Ok(aliases)
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn add_pipeline(&self, pipeline: String, name: String) -> Result<(), Error> {
        let pipeline: serde_json::Value =
            serde_json::from_str(&pipeline).context(JsonDeserialization {
                details: format!("Could not deserialize pipeline {}", name),
            })?;
        let response = self
            .0
            .ingest()
            .put_pipeline(IngestPutPipelineParts::Id(&name))
            .body(pipeline)
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot add pipeline '{}'", name,),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"acknowledged": Bool(true)})
            // We verify that acknowledge is true, then add the cat indices API to get the full index.
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let acknowledged = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("acknowledged")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'acknowledged'"),
                    json: json.clone(),
                })?
                .as_bool()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON boolean"),
                    json: json.clone(),
                })?;

            if acknowledged {
                Ok(())
            } else {
                Err(Error::NotAcknowledged {
                    details: format!("pipeline {} creation", name),
                })
            }
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn get_previous_indices(&self, index: &Index) -> Result<Vec<String>, Error> {
        let base_index = configuration::root_doctype_dataset(&index.doc_type, &index.dataset);
        // FIXME When available, we can use aliases.into_keys
        let aliases = self.find_aliases(base_index).await?;
        Ok(aliases
            .into_iter()
            .map(|(k, _)| k)
            .filter(|i| i.as_str() != index.name)
            .collect())
    }

    pub(super) async fn refresh_index(&self, index: String) -> Result<(), Error> {
        let response = self
            .0
            .indices()
            .refresh(IndicesRefreshParts::Index(&[&index]))
            .send()
            .await
            .context(ElasticsearchClient {
                details: format!("cannot refresh index {}", index),
            })?;

        // Note We won't analyze the details of the response.
        if !response.status_code().is_success() {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        } else {
            Ok(())
        }
    }

    pub(super) async fn list_documents<D>(
        &self,
        index: String,
    ) -> Result<Pin<Box<dyn Stream<Item = D> + Send>>, Error>
    where
        D: DeserializeOwned + Send + Sync + 'static,
    {
        let client = &self.0;
        let pit: String = {
            let response = client
                .open_point_in_time(OpenPointInTimeParts::Index(&[&index]))
                .keep_alive("1m")
                .send()
                .await
                .unwrap()
                .error_for_status_code()
                .context(ElasticsearchClient {
                    details: format!("cannot list index {}", index),
                })?;
            let response_body = response.json::<Value>().await.unwrap();
            response_body
                .get("id")
                .expect("response has id")
                .as_str()
                .unwrap()
                .into()
        };

        let client = self.0.clone();
        let stream = stream::unfold(State::Start, move |state| {
            let client = client.clone();
            let index = index.clone();
            let pit = pit.clone();
            async move {
                match state {
                    State::Start => {
                        // We're starting, so we get a pit, and make a first requestj
                        let body = json!({
                            "query": { "match_all": {}},
                            "pit": {"id": pit, "keep_alive": "1m" },
                            "sort": [ { "indexed_at": {"order": "asc" }}, {"_id": {"order": "asc"}} ],
                            "track_total_hits": false
                        });

                        let response = client
                            .search(SearchParts::None)
                            .body(body)
                            .send()
                            .await
                            .context(ElasticsearchClient {
                                details: format!("cannot search index {}", index),
                            })
                            .unwrap();

                        let response_body = response.json::<Value>().await.unwrap();

                        let pit = response_body
                            .get("pit_id")
                            .expect("response has pit_id")
                            .as_str()
                            .unwrap();

                        let hits = response_body
                            .get("hits")
                            .expect("response has hits")
                            .as_object()
                            .unwrap()["hits"]
                            .as_array()
                            .unwrap();

                        if hits.is_empty() {
                            Some((stream::iter(vec![]), State::End(String::from(pit))))
                        } else {
                            let last_hit = hits.last().unwrap();

                            let sort = last_hit
                                .as_object()
                                .unwrap()
                                .get("sort")
                                .expect("hit has sort")
                                .as_array()
                                .unwrap();

                            let timestamp = sort[0].as_u64().unwrap();
                            let tiebreaker = sort[1].as_str().unwrap();

                            let continuation_token = ContinuationToken {
                                pit: String::from(pit),
                                timestamp,
                                tiebreaker: tiebreaker.into(),
                            };

                            Some((
                                stream::iter(
                                    hits.to_owned()
                                        .into_iter()
                                        .map(|i| {
                                            let source = i
                                                .as_object()
                                                .unwrap()
                                                .get("_source")
                                                .expect("object has source")
                                                .to_owned();
                                            serde_json::from_value::<D>(source).unwrap()
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                                State::Next(continuation_token),
                            ))
                        }
                    }
                    State::Next(continuation_token) => {
                        let body = json!({
                            "query": { "match_all": {}},
                            "pit": {"id": continuation_token.pit, "keep_alive": "1m" },
                            "sort": [ { "indexed_at": {"order": "asc" }}, {"_id": {"order": "asc"}} ],
                            "search_after": [
                                continuation_token.timestamp,
                                continuation_token.tiebreaker
                            ]
                        });

                        let response = client
                            .search(SearchParts::None)
                            .body(body)
                            .send()
                            .await
                            .context(ElasticsearchClient {
                                details: format!("cannot search index {}", index),
                            })
                            .unwrap();

                        let response_body = response.json::<Value>().await.unwrap();
                        let pit = response_body["pit_id"].as_str().unwrap_or_else(|| panic!("Unexpected response: {}", response_body));

                        let hits = response_body["hits"].as_object().unwrap()["hits"]
                            .as_array()
                            .unwrap();

                        if hits.is_empty() {
                            Some((stream::iter(vec![]), State::End(String::from(pit))))
                        } else {
                            let last_hit = hits.last().unwrap();

                            let sort = last_hit.as_object().unwrap()["sort"].as_array().unwrap();

                            let timestamp = sort[0].as_u64().unwrap();
                            let tiebreaker = sort[1].as_str().unwrap();

                            let continuation_token = ContinuationToken {
                                pit: String::from(pit),
                                timestamp,
                                tiebreaker: tiebreaker.into(),
                            };
                            Some((
                                stream::iter(
                                    hits.to_owned()
                                        .into_iter()
                                        .map(|i| {
                                            let source = i
                                                .as_object()
                                                .unwrap()
                                                .get("_source")
                                                .expect("object has source")
                                                .to_owned();
                                            serde_json::from_value::<D>(source).unwrap()
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                                State::Next(continuation_token),
                            ))
                        }
                    }
                    State::End(pit) => {
                        let response = client
                            .close_point_in_time()
                            .body(json!({"id": pit}))
                            .send()
                            .await
                            .unwrap();

                        let _response_body = response.json::<Value>().await.unwrap();

                        None
                    }
                }
            }
        })
        .flatten();

        Ok(stream.boxed())
    }

    pub(super) async fn search_documents<D>(
        &self,
        indices: Vec<String>,
        query: Query,
    ) -> Result<Vec<D>, Error>
    where
        D: DeserializeOwned + Send + Sync + 'static,
    {
        let indices = indices.iter().map(String::as_str).collect::<Vec<_>>();

        let search = self.0.search(SearchParts::Index(&indices));

        let response = match query {
            Query::QueryString(q) => search.q(&q).send().await.context(ElasticsearchClient {
                details: format!("could not search indices {}", indices.join(", ")),
            })?,
            Query::QueryDSL(json) => {
                search
                    .body(json)
                    .send()
                    .await
                    .context(ElasticsearchClient {
                        details: format!("could not search indices {}", indices.join(", ")),
                    })?
            }
        };

        if response.status_code().is_success() {
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let hits = json
                .as_object()
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("hits")
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected 'hits'"),
                    json: json.clone(),
                })?
                .as_object()
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("hits")
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected 'hits'"),
                    json: json.clone(),
                })?
                .as_array()
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected JSON array"),
                    json: json.clone(),
                })?;

            let hits = hits
                .to_owned()
                .into_iter()
                .map(|i| {
                    let source = i
                        .as_object()
                        .unwrap()
                        .get("_source")
                        .expect("object has source")
                        .to_owned();
                    serde_json::from_value::<D>(source).unwrap()
                })
                .collect::<Vec<_>>();

            Ok(hits)
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn explain_search<D>(
        &self,
        index: String,
        query: Query,
        id: String,
    ) -> Result<D, Error>
    where
        D: DeserializeOwned + Send + Sync + 'static,
    {
        let explain = self.0.explain(ExplainParts::IndexId(&index, &id));

        let response = match query {
            Query::QueryString(q) => explain.q(&q).send().await.context(ElasticsearchClient {
                details: format!("could not explain document {} in index {}", id, index),
            })?,
            Query::QueryDSL(json) => {
                explain
                    .body(json)
                    .send()
                    .await
                    .context(ElasticsearchClient {
                        details: format!("could not explain document {} in index {}", id, index),
                    })?
            }
        };

        if response.status_code().is_success() {
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            println!("json: {:?}", json);

            let explanation = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("explanation")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'hits'"),
                    json: json.clone(),
                })?
                .to_owned();
            let explanation =
                serde_json::from_value::<D>(explanation).context(JsonDeserialization {
                    details: String::from("could not deserialize explanation"),
                })?;
            Ok(explanation)
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn cluster_health(&self) -> Result<StorageHealth, Error> {
        let response = self
            .0
            .cluster()
            .health(ClusterHealthParts::None)
            .send()
            .await
            .context(ElasticsearchClient {
                details: String::from("cannot query cluster health"),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // Object({"cluster_name": "foo", "status": "yellow", ...})
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let health = json
                .as_object()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })?
                .get("status")
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected 'status'"),
                    json: json.clone(),
                })?
                .as_str()
                .ok_or_else(|| Error::JsonInvalid {
                    details: String::from("expected JSON string"),
                    json: json.clone(),
                })?;

            match health {
                "green" | "yellow" => Ok(StorageHealth::OK),
                "red" => Ok(StorageHealth::FAIL),
                _ => Err(Error::ElasticsearchUnhandledStatus {
                    details: health.to_string(),
                }),
            }
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }

    pub(super) async fn cluster_version(&self) -> Result<StorageVersion, Error> {
        // In the following, we specify the list of columns we're interested in ("v" for version).
        // Refer to https://www.elastic.co/guide/en/elasticsearch/reference/current/cat-nodes.html
        // to explicitely set the list of columns
        let response = self
            .0
            .cat()
            .nodes()
            .h(&["v"]) // We only want the version
            .format("json")
            .send()
            .await
            .context(ElasticsearchClient {
                details: String::from("cannot query cluster health"),
            })?;

        if response.status_code().is_success() {
            let json = response
                .json::<Value>()
                .await
                .context(ElasticsearchDeserialization)?;

            let version = json
                .as_array()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON array"),
                    json: json.clone(),
                })?
                .get(0)
                .ok_or(Error::JsonInvalid {
                    details: String::from("empty list of node information"),
                    json: json.clone(),
                })?
                .get("v")
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected 'v' (version)"),
                    json: json.clone(),
                })?
                .as_str()
                .ok_or(Error::JsonInvalid {
                    details: String::from("expected JSON string"),
                    json: json.clone(),
                })?;
            Ok(version.to_string())
        } else {
            let exception = response.exception().await.ok().unwrap();

            match exception {
                Some(exception) => {
                    let err = Error::from(exception);
                    Err(err)
                }
                None => Err(Error::ElasticsearchFailureWithoutException {
                    details: String::from("Fail status without exception"),
                }),
            }
        }
    }
}

/// This is the information provided by Elasticsearch CAT Indice API
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ElasticsearchIndex {
    pub(crate) health: String,
    pub status: String,
    #[serde(rename = "index")]
    pub(crate) name: String,
    #[serde(rename = "docs.count")]
    pub(crate) docs_count: Option<String>,
    #[serde(rename = "docs.deleted")]
    pub(crate) docs_deleted: Option<String>,
    pub(crate) pri: String,
    #[serde(rename = "pri.store.size")]
    pub(crate) pri_store_size: Option<String>,
    pub(crate) rep: String,
    #[serde(rename = "store.size")]
    pub(crate) store_size: Option<String>,
    pub(crate) uuid: String,
}

impl TryFrom<ElasticsearchIndex> for Index {
    type Error = Error;
    fn try_from(index: ElasticsearchIndex) -> Result<Self, Self::Error> {
        let ElasticsearchIndex {
            name,
            docs_count,
            status,
            ..
        } = index;
        let (doc_type, dataset) =
            configuration::split_index_name(&name).map_err(|err| Error::IndexConversion {
                details: format!(
                    "could not convert elasticsearch index into model index: {}",
                    err.to_string()
                ),
            })?;

        let docs_count = match docs_count {
            Some(val) => val.parse::<u32>().expect("docs count"),
            None => 0,
        };
        Ok(Index {
            name,
            doc_type,
            dataset,
            docs_count,
            status: IndexStatus::from(status),
        })
    }
}

impl From<String> for IndexStatus {
    fn from(status: String) -> Self {
        match status.as_str() {
            "green" => IndexStatus::Available,
            "yellow" => IndexStatus::Available,
            _ => IndexStatus::Available,
        }
    }
}

struct ContinuationToken {
    pit: String,
    timestamp: u64,
    tiebreaker: String,
}

enum State {
    Start,
    Next(ContinuationToken),
    End(String),
}

#[derive(Debug, Default)]
pub struct InsertStats {
    pub(crate) created: usize,
    pub(crate) updated: usize,
    pub(crate) error: usize,
}

impl From<InsertStats> for ModelInsertStats {
    fn from(stats: InsertStats) -> Self {
        let InsertStats {
            created,
            updated,
            error,
        } = stats;
        ModelInsertStats {
            created,
            updated,
            error,
        }
    }
}

impl TryFrom<String> for StorageHealth {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "green" | "yellow" => Ok(StorageHealth::OK),
            "red" => Ok(StorageHealth::FAIL),
            _ => Err(Error::ElasticsearchUnhandledStatus { details: value }),
        }
    }
}
