use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use futures::stream::TryStreamExt;

use mimir2::adapters::secondary::elasticsearch::{
    remote::connection_test_pool, ElasticsearchStorageConfig,
};
use mimir2::domain::ports::primary::list_documents::ListDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use mimir2::utils::docker;
use places::addr::Addr;
use tests::{bano, cosmogony, download};

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(6)
        .enable_time()
        .enable_io()
        .build()
        .unwrap();

    rt.block_on(async {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");
        let config = ElasticsearchStorageConfig::default_testing();
        let client = connection_test_pool()
            .conn(config)
            .await
            .expect("could not establish connection with Elasticsearch");

        download::osm("ile-de-france").await.unwrap();
        // false: don't force regenerate admins for 'ile-de-france'
        cosmogony::generate("ile-de-france", false).await.unwrap();
        // true: force reindex admins on bench dataset for 'ile-de-france'
        cosmogony::index_admins(&client, "ile-de-france", "bench", true)
            .await
            .unwrap();
        download::bano("ile-de-france", &["75", "77", "78", "92", "93", "94", "95"])
            .await
            .unwrap();
        bano::index_addresses(&client, "ile-de-france", "bench", true)
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("listing");
    group.bench_function("listing addresses", |b| {
        b.iter(|| {
            rt.block_on(async move {
                let config = ElasticsearchStorageConfig::default_testing();
                let client = connection_test_pool()
                    .conn(config)
                    .await
                    .expect("could not establish connection with Elasticsearch");
                let addresses: Vec<Addr> = client
                    .list_documents()
                    .await
                    .unwrap()
                    .try_collect()
                    .await
                    .unwrap();
                println!("address count: {}", addresses.len());
            })
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    // config = Criterion::default().significance_level(0.1).sample_size(5);
    config = Criterion::default();
    targets = bench
}
criterion_main!(benches);
