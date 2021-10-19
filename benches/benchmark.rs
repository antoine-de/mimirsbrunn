use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use futures::stream::TryStreamExt;

use mimir2::adapters::secondary::elasticsearch::remote::connection_pool_url;
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use mimir2::utils::docker::ConfigElasticsearchTesting;
use places::admin::Admin;
use tests::{cosmogony, download};

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    rt.block_on(async {
        let config = ConfigElasticsearchTesting::default();
        let pool = connection_pool_url(&config.url)
            .await
            .expect("could not initialize Elasticsearch Connection Pool");
        let client = pool
            .conn(config.timeout, &config.version_req)
            .await
            .expect("could not establish connection with Elasticsearch");

        download::osm("ile-de-france").await.unwrap();
        // don't force regenerate admins for 'ile-de-france'
        cosmogony::generate("ile-de-france", false).await.unwrap();
        // force reindex admins on bench dataset for 'ile-de-france'
        cosmogony::index_admins(&client, "ile-de-france", "bench", true)
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("sample-size-example");
    group.bench_function("listing admins", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let config = ConfigElasticsearchTesting::default();
                let pool = connection_pool_url(&config.url)
                    .await
                    .expect("could not initialize Elasticsearch Connection Pool");
                let client = pool
                    .conn(config.timeout, &config.version_req)
                    .await
                    .expect("could not establish connection with Elasticsearch");
                let admins: Vec<Admin> = client
                    .list_documents()
                    .await
                    .unwrap()
                    .try_collect()
                    .await
                    .unwrap();
                println!("admin count: {}", admins.len());
            })
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(20);
    targets = bench
}
criterion_main!(benches);
