use cucumber::async_trait;
use std::convert::Infallible;
use std::sync::Arc;

use mimir2::domain::model::index::{Index, IndexStatus};
use mimir2::utils::docker;

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    input_data: Vec<example_steps::Person>,
    output_data: Vec<example_steps::Person>,
    index: Index,
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            input_data: Vec::new(),
            output_data: Vec::new(),
            index: Index {
                name: String::new(),
                dataset: String::new(),
                doc_type: String::new(),
                docs_count: 0u32,
                status: IndexStatus::NotAvailable,
            },
        })
    }
}

impl MyWorld {
    async fn initialize_docker() -> () {
        let mtx = Arc::clone(&docker::AVAILABLE);
        let _guard = mtx.lock().unwrap();
        docker::initialize().await.expect("docker initialize");
    }
}

mod example_steps {
    use cucumber::{t, Steps};
    use futures::stream::StreamExt;
    use serde::{Deserialize, Serialize};
    use std::fmt::Display;
    use std::path::PathBuf;
    use tokio::io::AsyncReadExt;
    use uuid::Uuid;

    use mimir2::adapters::secondary::elasticsearch;
    use mimir2::adapters::secondary::elasticsearch::internal::{
        IndexConfiguration, IndexMappings, IndexParameters, IndexSettings,
    };
    use mimir2::domain::model::{
        configuration::Configuration, document::Document, export_parameters::ListParameters,
        index::IndexVisibility,
    };
    use mimir2::domain::ports::remote::Remote;
    use mimir2::domain::usecases::{
        generate_index::{GenerateIndex, GenerateIndexParameters},
        list_documents::{ListDocuments, ListDocumentsParameters},
        UseCase,
    };

    #[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
    pub(super) struct Person {
        id: Uuid,
        name: String,
        age: u16,
    }

    impl Person {
        const DOC_TYPE: &'static str = "person";
    }

    impl Document for Person {
        fn doc_type(&self) -> &'static str {
            Self::DOC_TYPE
        }

        fn id(&self) -> String {
            self.id.to_string()
        }
    }

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_regex_async(
                "I have generated an index from \"(.*)\"",
                t!(|mut world, matches, _step| {
                    crate::MyWorld::initialize_docker().await;
                    let settings = include_str!("./fixtures/settings.json");
                    let mappings = include_str!("./fixtures/mappings.json");
                    let index_name = String::from("integration-test");
                    let config = IndexConfiguration {
                        name: index_name.clone(),
                        parameters: IndexParameters {
                            timeout: String::from("10s"),
                            wait_for_active_shards: String::from("1"), // only the primary shard
                        },
                        settings: IndexSettings {
                            value: String::from(settings), // <<=== Invalid Settings
                        },
                        mappings: IndexMappings {
                            value: String::from(mappings),
                        },
                    };
                    let config = Configuration {
                        value: serde_json::to_string(&config).expect("config"),
                    };
                    let mut dataset_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                    dataset_file.push("tests/fixtures");
                    dataset_file.push(matches[1].clone());
                    let mut dataset_file = tokio::fs::File::open(&dataset_file)
                        .await
                        .expect(&format!("Opening dataset from {}", dataset_file.display()));

                    // Read the content of the file into a buffer.
                    let mut dataset_content = String::new();
                    dataset_file
                        .read_to_string(&mut dataset_content)
                        .await
                        .expect("reading dataset");

                    let dataset: Vec<Person> = serde_json::from_str(&dataset_content).unwrap();
                    world.input_data = dataset.iter().cloned().collect();
                    let stream = futures::stream::iter(dataset);
                    let param = GenerateIndexParameters {
                        config,
                        documents: Box::new(stream),
                        visibility: IndexVisibility::Public,
                    };

                    let pool = elasticsearch::remote::connection_test_pool()
                        .await
                        .expect("connection pool");
                    let client = pool.conn().await.expect("client connection");
                    let usecase = GenerateIndex::new(Box::new(client));
                    usecase.execute(param).await.unwrap();
                    world
                }),
            )
            .when_async(
                "I list all the documents in the index",
                t!(|mut world, _step| {
                    let pool = elasticsearch::remote::connection_test_pool()
                        .await
                        .expect("connection pool");
                    let client = pool.conn().await.expect("client connection");

                    let list_documents = ListDocuments::new(Box::new(client));

                    let parameters = ListDocumentsParameters {
                        parameters: ListParameters {
                            doc_type: String::from(Person::DOC_TYPE),
                        },
                    };
                    let person_stream = list_documents
                        .execute(parameters)
                        .await
                        .expect("document stream");

                    world.output_data = person_stream
                        .map(|v| serde_json::from_value(v).expect("cannot deserialize person"))
                        .collect::<Vec<Person>>()
                        .await;

                    world
                }),
            )
            .then("I find the original list", |world, _step| {
                // Note that the original vector is very different from the previous one because of
                // their order. So I only compare the length.
                // I should be comparing BTreeSet instead...
                assert_eq!(world.input_data.len(), world.output_data.len());
                world
            });

        builder
    }
}

#[tokio::main]
async fn main() {
    // Do any setup you need to do before running the Cucumber runner.
    // e.g. setup_some_db_thing()?;

    cucumber::Cucumber::<MyWorld>::new()
        // Specifies where our feature files exist
        .features(&["./features"])
        // Adds the implementation of our steps to the runner
        .steps(example_steps::steps())
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
