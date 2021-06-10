use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use rand::distributions::{Alphanumeric, Distribution, Uniform};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use uuid::Uuid;

use mimir2::adapters::secondary::elasticsearch;
use mimir2::adapters::secondary::elasticsearch::internal::{
    IndexConfiguration, IndexMappings, IndexParameters, IndexSettings,
};
use mimir2::domain::model::configuration::Configuration;
use mimir2::domain::model::document::Document;
use mimir2::domain::model::index::IndexVisibility;
use mimir2::domain::ports::remote::Remote;
use mimir2::domain::ports::storage::Storage;
use mimir2::domain::usecases::generate_index::GenerateIndex;
use mimir2::domain::usecases::generate_index::GenerateIndexParameters;
use mimir2::domain::usecases::UseCase;

fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    c.bench_function("people", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let settings = include_str!("./fixtures/settings.json");
                let mappings = include_str!("./fixtures/mappings.json");
                let index_name = String::from("test-benchmark-people");
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
                let pool = elasticsearch::remote::connection_test_pool()
                    .await
                    .expect("connection pool");
                let client = pool.conn().await.expect("client connection");
                let _ = client.delete_container(String::from("*")).await.unwrap();
            })
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[derive(Deserialize, Serialize)]
struct Person {
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

fn generate_people(count: usize) -> Vec<Person> {
    let mut rng = rand::thread_rng();
    let age_gen = Uniform::from(9..99);
    let name_len_gen = Uniform::from(5..20);
    let mut ps = Vec::with_capacity(count);
    for _ in 0..count {
        let age = age_gen.sample(&mut rng);
        let name_len = name_len_gen.sample(&mut rng);
        let name: String = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(name_len)
            .collect();
        let id = Uuid::new_v4();
        let p = Person { id, name, age };
        ps.push(p);
    }
    ps
}
