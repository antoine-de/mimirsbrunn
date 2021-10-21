use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use async_trait::async_trait;
use common::document::ContainerDocument;
use cucumber::{t, StepContext, Steps};
use mimir2::adapters::primary::{
    common::dsl::build_query, common::filters::Filters, common::settings::QuerySettings,
};
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir2::domain::{model::query::Query, ports::primary::search_documents::SearchDocuments};
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.when_regex_async(
        "the user searches for \"(.*)\"",
        t!(|mut state, ctx| {
            let domain = vec![
                Addr::static_doc_type().to_string(),
                Admin::static_doc_type().to_string(),
                Street::static_doc_type().to_string(),
                Stop::static_doc_type().to_string(),
                Poi::static_doc_type().to_string(),
            ];

            let query = ctx.matches[1].clone();

            state
                .execute(Search::new(domain, query), &ctx)
                .await
                .expect("failed to search");

            state
        }),
    );

    steps.when_regex_async(
        "the user searches \"(.*)\" for \"(.*)\"",
        t!(|mut state, ctx| {
            let domain: Vec<String> = ctx.matches[1]
                .clone()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let query = ctx.matches[2].clone();

            state
                .execute(Search::new(domain, query), &ctx)
                .await
                .expect("failed to search");

            state
        }),
    );

    steps.then_regex_async(
        "he finds \"(.*)\" as the first result",
        t!(|mut state, ctx| {
            let id = ctx.matches[1].clone();

            state
                .execute(HasDocument { id, max_rank: 1 }, &ctx)
                .await
                .expect("failed to find document");

            state
        }),
    );

    steps.then_regex_async(
        "he finds \"(.*)\", \"(.*)\", \"(.*)\", and \"(.*)\" in the first \"(.*)\" results",
        t!(|mut state, ctx| {
            let house = Some(ctx.matches[1].clone()).and_then(|house| {
                if house.is_empty() {
                    None
                } else {
                    Some(house)
                }
            });
            let street = Some(ctx.matches[2].clone()).and_then(|street| {
                if street.is_empty() {
                    None
                } else {
                    Some(street)
                }
            });
            let city = Some(ctx.matches[3].clone()).and_then(|city| {
                if city.is_empty() {
                    None
                } else {
                    Some(city)
                }
            });
            let postcode = Some(ctx.matches[4].clone()).and_then(|postcode| {
                if postcode.is_empty() {
                    None
                } else {
                    Some(postcode)
                }
            });
            let limit = ctx.matches[5].clone();
            let limit = if limit.is_empty() {
                1
            } else {
                limit.parse().expect("limit as usize")
            };

            state
                .execute(
                    HasAddress {
                        house,
                        street,
                        city,
                        postcode,
                        limit,
                    },
                    &ctx,
                )
                .await
                .expect("failed to find document");

            state
        }),
    );

    steps
}

/// Perform a search in current Elasticsearch DB.
pub struct Search {
    domain: Vec<String>,
    query: String,
    results: Vec<serde_json::Value>,
}

impl Search {
    pub fn new(domain: Vec<String>, query: String) -> Self {
        Self {
            domain,
            query,
            results: Vec::new(),
        }
    }
}

#[async_trait(?Send)]
impl Step for Search {
    async fn execute(&mut self, _state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        // Build ES query
        let dsl = build_query(
            &self.query,
            Filters::default(),
            &["fr"],
            &QuerySettings::default(),
        );

        // Fetch documents
        self.results = {
            client
                .search_documents(self.domain.clone(), Query::QueryDSL(dsl))
                .await
                .unwrap()
        };

        Ok(StepStatus::Done)
    }
}

/// Check if given document is in the output.
///
/// It assumes that a Search has already been performed before.
pub struct HasDocument {
    id: String,
    max_rank: usize,
}

#[async_trait(?Send)]
impl Step for HasDocument {
    async fn execute(&mut self, state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let (search, _) = state
            .steps_for::<Search>()
            .next_back()
            .expect("the user must perform a search before checking results");

        let (rank, _) = (search.results.iter().enumerate())
            .find(|(_, doc)| {
                println!("{} vs {}", self.id, doc["id"].as_str().unwrap());
                self.id == doc["id"].as_str().unwrap()
            })
            .expect("document was not found in search results");

        assert!(rank < self.max_rank);
        Ok(StepStatus::Done)
    }
}

/// Check if given document is in the output.
///
/// It assumes that a Search has already been performed before.
pub struct HasAddress {
    house: Option<String>,
    street: Option<String>,
    city: Option<String>,
    postcode: Option<String>,
    limit: usize,
}

#[async_trait(?Send)]
impl Step for HasAddress {
    async fn execute(&mut self, state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let (search, _) = state
            .steps_for::<Search>()
            .next_back()
            .expect("the user must perform a search before checking results");

        let (rank, _) = (search.results.iter().enumerate())
            .find(|(_, doc)| {
                if let Some(house) = &self.house {
                    if house != doc["house_number"].as_str().unwrap() {
                        return false;
                    }
                }
                if let Some(street) = &self.street {
                    if street != doc["street"]["name"].as_str().unwrap() {
                        return false;
                    }
                }
                // FIXME Need city check. Maybe add a city method to an address
                // if let Some(city) = self.street {
                //     if street != doc["street"]["name"] {
                //         return false;
                //     }
                // }
                true
            })
            .expect("document was not found in search results");

        assert!(rank < self.limit);
        Ok(StepStatus::Done)
    }
}
