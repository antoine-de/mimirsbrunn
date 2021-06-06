use crate::bano::Bano;
use crate::Error;
use failure::format_err;
use futures::future;
use futures::stream::{Stream, StreamExt};
use mimir::Addr;
use mimir2::{
    adapters::secondary::elasticsearch::{internal::IndexConfiguration, ElasticsearchStorage},
    domain::{
        model::{configuration::Configuration, document::Document, index::IndexVisibility},
        usecases::{
            generate_index::{GenerateIndex, GenerateIndexParameters},
            UseCase,
        },
    },
};
use serde::Serialize;
use slog_scope::warn;
use std::marker::{Send, Sync};
use std::path::PathBuf;
use tokio::fs::File;

// We use a new type to wrap around Addr and implement the Document trait.
#[derive(Serialize)]
struct AddrDoc(Addr);

impl Document for AddrDoc {
    const IS_GEO_DATA: bool = true;
    const DOC_TYPE: &'static str = "addr";
    fn id(&self) -> String {
        self.0.id.clone()
    }
}

async fn import_addresses<S, F>(
    client: ElasticsearchStorage,
    config: IndexConfiguration,
    records: S,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(Bano) -> Result<Addr, Error> + Send + Sync + 'static,
    S: Stream<Item = Bano> + Send + Sync + Unpin + 'static,
{
    // let addr_index = rubber
    //     .make_index(dataset, &index_settings)
    //     .with_context(|err| format!("Error occurred when making index {}: {}", dataset, err))?;

    // info!("Add data in elasticsearch db.");

    // let mut country_stats: Arc<Mutex<HashMap<String, i32>>> = Arc::new(Mutex::new(HashMap::new()));

    let addrs = records.map(into_addr).filter_map(|ra| match ra {
        Ok(a) => {
            if a.street.name.is_empty() {
                warn!("Address {} has no street name and has been ignored.", a.id);
                future::ready(None)
            } else {
                future::ready(Some(AddrDoc(a)))
            }
        }
        Err(err) => {
            warn!("Address Error ignored: {}", err);
            future::ready(None)
        }
    });
    // .inspect(|addr| {
    //     let country_code = addr
    //         .0
    //         .country_codes
    //         .first()
    //         .map(|string| string.as_str())
    //         .unwrap_or("other");

    //     let mut z = country_stats.lock().unwrap();
    //     if let Some(count) = z.get_mut(country_code) {
    //         *count += 1;
    //     } else {
    //         country_stats
    //             .lock()
    //             .unwrap()
    //             .insert(country_code.to_string(), 1);
    //     }
    // });

    let config = serde_json::to_string(&config).map_err(|err| {
        format_err!(
            "could not serialize index configuration: {}",
            err.to_string()
        )
    })?;
    let generate_index = GenerateIndex::new(Box::new(client));
    let parameters = GenerateIndexParameters {
        config: Configuration { value: config },
        documents: Box::new(addrs),
        visibility: IndexVisibility::Public,
    };
    generate_index
        .execute(parameters)
        .await
        .map_err(|err| format_err!("could not generate index: {}", err.to_string()))?;

    // info!("Addresses imported per country:");
    // let z = country_stats.lock().unwrap();
    // let mut country_stats: Vec<_> = z.iter().collect();
    // country_stats.sort_unstable_by_key(|(_, count)| *count);

    // for (country, count) in country_stats.into_iter().rev() {
    //     info!("{:>10} {}", country, count);
    // }

    Ok(())
}

/* pub async fn import_addresses_from_stream<S, T, F>(
    client: ElasticsearchStorage,
    config: IndexConfiguration,
    rubber: &mut Rubber,
    has_headers: bool,
    nb_threads: usize,
    streams: impl IntoIterator<Item = impl Read>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
    S: Stream<Item = > + Send + Sync + 'static,
{
    let iter = streams
        .into_iter()
        .flat_map(|stream| {
            csv::ReaderBuilder::new()
                .has_headers(has_headers)
                .from_reader(stream)
                .into_deserialize()
        })
        .filter_map(|line| {
            line.map_err(|e| warn!("Impossible to read line, error: {}", e))
                .ok()
        });

    import_addresses(client, config, rubber, nb_threads, iter, into_addr).await
}

pub async fn import_addresses_from_files<T, F>(
    client: ElasticsearchStorage,
    config: IndexConfiguration,
    rubber: &mut Rubber,
    has_headers: bool,
    nb_threads: usize,
    files: impl IntoIterator<Item = PathBuf>,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(T) -> Result<Addr, Error> + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let stream = futures::stream::iter(files.into_iter()).filter_map(|path| {
        info!("importing {:?}...", &path);

        let with_gzip = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "gz")
            .unwrap_or(false);

        File::open(&path)
            .map_err(|err| error!("Impossible to read file {:?}, error: {}", path, err))
            .map(|file| {
                if with_gzip {
                    let decoder = GzDecoder::new(file);
                    Box::new(decoder) as Box<dyn Read>
                } else {
                    Box::new(file) as Box<dyn Read>
                }
            })
            .ok()
    });

    import_addresses_from_streams(
        client,
        config,
        rubber,
        has_headers,
        nb_threads,
        stream,
        into_addr,
    )
    .await
} */

pub async fn import_addresses_from_file<F>(
    client: ElasticsearchStorage,
    config: IndexConfiguration,
    file: PathBuf,
    into_addr: F,
) -> Result<(), Error>
where
    F: Fn(Bano) -> Result<Addr, Error> + Send + Sync + 'static,
{
    let reader = csv_async::AsyncReaderBuilder::new().create_deserializer(File::open(file).await?);
    let records = reader
        .into_deserialize::<Bano>()
        .filter_map(|rec| future::ready(rec.ok()));

    import_addresses(client, config, records, into_addr).await
}
