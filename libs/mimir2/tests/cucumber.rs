use cucumber::async_trait;
use std::convert::Infallible;

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    input_data: Vec<example_steps::Person>,
    output_data: Vec<example_steps::Person>,
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            input_data: Vec::new(),
            output_data: Vec::new(),
        })
    }
}

mod example_steps {
    use cucumber::{t, Steps};
    use futures::stream::StreamExt;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use mimir2::adapters::secondary::elasticsearch;
    use mimir2::adapters::secondary::elasticsearch::internal::{
        IndexConfiguration, IndexMappings, IndexParameters, IndexSettings,
    };
    use mimir2::domain::model::{
        configuration::Configuration, document::Document, index::IndexVisibility,
        query_parameters::QueryParameters,
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

    impl Document for Person {
        const IS_GEO_DATA: bool = false;
        const DOC_TYPE: &'static str = "person";

        fn id(&self) -> String {
            self.id.to_string()
        }
    }

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_async(
                "I have generated an index",
                t!(|mut world, _step| {
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
                    let data = include_str!("./fixtures/data.json");
                    let data: Vec<Person> = serde_json::from_str(data).unwrap();
                    world.input_data = data.iter().cloned().collect();
                    let stream = futures::stream::iter(data);
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
                        query_parameters: QueryParameters {
                            containers: vec![String::from("munin_person")],
                            dsl: String::from(r#"{ "match_all": {} }"#),
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
