use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use itertools::{EitherOrBoth::*, Itertools};
use std::cmp::Ordering;

use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use common::document::ContainerDocument;
use mimir2::adapters::primary::bragi::api::DEFAULT_LIMIT_RESULT_ES;
use mimir2::adapters::primary::{
    common::coord::Coord, common::dsl::build_query, common::filters::Filters,
    common::settings::QuerySettings,
};
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir2::domain::{model::query::Query, ports::primary::search_documents::SearchDocuments};
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.when_regex_async(
        r#"the user searches (.*) datatype(?: in (.*))? for "(.*)"(?: at \((.*),(.*)\))?"#,
        t!(|mut state, ctx| {
            let places = ctx.matches[1].clone();
            let places = if places == "all" {
                vec![
                    Addr::static_doc_type().to_string(),
                    Admin::static_doc_type().to_string(),
                    Street::static_doc_type().to_string(),
                    Stop::static_doc_type().to_string(),
                    Poi::static_doc_type().to_string(),
                ]
            } else {
                places.split(',').map(|s| s.trim().to_string()).collect()
            };
            let lat = ctx.matches[4].trim();
            let lon = ctx.matches[5].trim();
            let coord = if lat.is_empty() {
                None
            } else {
                // round to 4 digits
                let lat = (lat.parse::<f32>().expect("lat is f32") * 10000.0).round() / 10000.0;
                let lon = (lon.parse::<f32>().expect("lon is f32") * 10000.0).round() / 10000.0;
                Some(Coord::new(lat, lon))
            };
            let filters = Filters {
                coord,
                ..Default::default()
            };

            let query = ctx.matches[3].clone();
            let search = Search {
                filters,
                places,
                query,
                results: Vec::new(),
            };

            state.execute(search, &ctx).await.expect("failed to search");

            state
        }),
    );

    steps.then_regex_async(
        r#"he finds "(.*)" as the first result"#,
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
        r#"he finds admin "(.*)", a "(.*)", in the first (.*) results"#,
        t!(|mut state, ctx| {
            let name = Some(ctx.matches[1].clone()).and_then(|name| {
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            });
            //let zone_type = Some(ctx.matches[2].clone()).and_then(|zone_type| {
            //    if zone_type.is_empty() {
            //        None
            //    } else {
            //        Some(zone_type)
            //    }
            //});
            let limit = ctx.matches[3].clone();
            let limit = if limit.is_empty() {
                1
            } else {
                limit.parse().expect("limit as usize")
            };

            state
                .execute(
                    HasAdmin {
                        name,
                        // zone_type,
                        zone_type: None,
                        limit,
                    },
                    &ctx,
                )
                .await
                .expect("failed to find document");

            state
        }),
    );

    steps.then_regex_async(
        r#"he finds address "(.*)", "(.*)", "(.*)", and "(.*)" in the first "(.*)" results"#,
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
#[derive(Debug)]
pub struct Search {
    places: Vec<String>, // What kind of places we're searching
    query: String,       // The search string
    filters: Filters,    // Search filters
    results: Vec<serde_json::Value>,
}

#[async_trait(?Send)]
impl Step for Search {
    async fn execute(&mut self, _state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");
        // Build ES query
        let dsl = build_query(
            &self.query,
            self.filters.clone(),
            &["fr"],
            &QuerySettings::default(),
        );

        // Fetch documents
        self.results = {
            client
                .search_documents(
                    self.places.clone(),
                    Query::QueryDSL(dsl),
                    DEFAULT_LIMIT_RESULT_ES,
                )
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

/// Check if given admin is in the output.
///
/// It assumes that a Search has already been performed before.
pub struct HasAdmin {
    name: Option<String>,
    zone_type: Option<String>,
    limit: usize,
}

#[async_trait(?Send)]
impl Step for HasAdmin {
    async fn execute(&mut self, state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let (search, _) = state
            .steps_for::<Search>()
            .next_back()
            .expect("the user must perform a search before checking results");

        let rank = search
            .results
            .clone()
            .into_iter()
            .enumerate()
            .find_map(|(num, doc)| {
                let admin: Admin = serde_json::from_value(doc).expect("admin");
                if let Some(name) = &self.name {
                    if !equal_ignore_case_utf8(name, admin.name.as_str()) {
                        return None;
                    }
                }
                if let Some(zone_type) = &self.zone_type {
                    if !equal_ignore_case_utf8(
                        zone_type,
                        admin.zone_type.expect("zone type").as_str(),
                    ) {
                        // is it justified to expect a zone type?
                        return None;
                    }
                }
                Some(num)
            })
            .expect("document was not found in search results");

        assert!(rank < self.limit);
        Ok(StepStatus::Done)
    }
}

/// Check if given address is in the output.
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

        let rank = search
            .results
            .clone()
            .into_iter()
            .enumerate()
            .find_map(|(num, doc)| {
                let addr: Addr = serde_json::from_value(doc).expect("address");
                if let Some(house) = &self.house {
                    if !equal_ignore_case_utf8(house, addr.house_number.as_str()) {
                        return None;
                    }
                }
                if let Some(street) = &self.street {
                    if !equal_ignore_case_utf8(street, addr.street.name.as_str()) {
                        return None;
                    }
                }
                if let Some(postcode) = &self.postcode {
                    if !equal_ignore_case_utf8(postcode, addr.zip_codes[0].as_str()) {
                        return None;
                    }
                }
                if let Some(city) = &self.city {
                    if !equal_ignore_case_utf8(city, addr.city().expect("city").as_str()) {
                        return None;
                    }
                }
                Some(num)
            })
            .expect("document was not found in search results");

        assert!(rank < self.limit);
        Ok(StepStatus::Done)
    }
}

// Based on https://stackoverflow.com/questions/63871601/what-is-an-efficient-way-to-compare-strings-while-ignoring-case
// Its a case insensitive comparison, returning true if equal, false otherwise
fn equal_ignore_case_utf8(a: &str, b: &str) -> bool {
    let abs = a
        .chars()
        .flat_map(char::to_lowercase)
        .zip_longest(b.chars().flat_map(char::to_lowercase));

    for ab in abs {
        match ab {
            Left(_) => return false,
            Right(_) => return false,
            Both(a, b) => {
                if a.cmp(&b) != Ordering::Equal {
                    return false;
                }
            }
        }
    }
    true
}
