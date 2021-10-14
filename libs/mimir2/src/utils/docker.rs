use bollard::service::{HostConfig, PortBinding};
use bollard::{
    container::{
        Config as BollardConfig, CreateContainerOptions, ListContainersOptions,
        StartContainerOptions,
    },
    errors::Error as BollardError,
    image::CreateImageOptions,
    Docker,
};
use config::Config;
use elasticsearch::{
    http::transport::BuildError as TransportBuilderError,
    indices::{IndicesDeleteAliasParts, IndicesDeleteIndexTemplateParts, IndicesDeleteParts},
    Error as ElasticsearchError,
};
use futures::stream::TryStreamExt;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use url::Url;

use crate::adapters::secondary::elasticsearch::remote::{self, Error as ElasticsearchRemoteError};
use crate::adapters::secondary::elasticsearch::ElasticsearchStorageConfig;
use crate::domain::ports::secondary::remote::{Error as RemoteError, Remote};

const DOCKER_ES_VERSION: &str = "7.13.0";

pub async fn initialize() -> Result<(), Error> {
    initialize_with_param(true).await
}

/// Initializes a docker container for testing
/// It will see if a docker container is available with the default name
/// If there is no container, it will create one.
/// If there is already a container, and the parameter cleanup is true,
/// then all the indices found on that Elasticsearch are wiped out.
/// Once the container is available, a connection is attempted, to make
/// sure subsequent calls to that Elasticsearch will be successful.
pub async fn initialize_with_param(cleanup: bool) -> Result<(), Error> {
    let mut docker = DockerWrapper::new();
    let is_available = docker.is_container_available().await?;
    if !is_available {
        docker.create_container().await?;
    } else if cleanup {
        docker.cleanup().await?;
    }
    let is_available = docker.is_container_available().await?;
    if !is_available {
        return Err(Error::Misc {
            msg: format!("Cannot get docker {} available", docker.docker_image),
        });
    }
    let pool = remote::connection_pool_url(docker.config.url.as_str())
        .await
        .context(ElasticsearchPoolCreation)?;
    let _client = pool
        .conn(Default::default())
        .await
        .context(ElasticsearchConnection)?;
    Ok(())
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Connection to docker socket: {}", source))]
    DockerConnection { source: BollardError },

    #[snafu(display("Creation of elasticsearch pool: {}", source))]
    ElasticsearchPoolCreation { source: ElasticsearchRemoteError },

    #[snafu(display("Connection to elasticsearch: {}", source))]
    ElasticsearchConnection { source: RemoteError },

    #[snafu(display("docker version: {}", source))]
    Version { source: BollardError },

    #[snafu(display("url parsing error: {}", source))]
    UrlParse { source: url::ParseError },

    #[snafu(display("elasticsearch transport error: {}", source))]
    ElasticsearchTransport { source: TransportBuilderError },

    #[snafu(display("elasticsearch client error: {}", source))]
    ElasticsearchClient { source: ElasticsearchError },

    #[snafu(display("docker error: {}", source))]
    DockerEngine { source: BollardError },

    #[snafu(display("error: {}", msg))]
    Misc { msg: String },
}

pub struct DockerWrapper {
    ports: Vec<(u32, u32)>, // list of ports to publish (host port, container port)
    docker_image: String,
    container_name: String, // ip: String,
    config: ElasticsearchStorageConfig,
}

impl Default for DockerWrapper {
    fn default() -> Self {
        let config = ElasticsearchStorageConfig::default();

        let docker_image = format!(
            "docker.elastic.co/elasticsearch/elasticsearch:{}",
            DOCKER_ES_VERSION
        );

        let port = config
            .url
            .port()
            .expect("expected port in elasticsearch url");

        let offset: u32 = (port - 9000).into();
        DockerWrapper {
            ports: vec![(9000 + offset, 9200), (9300 + offset, 9300)],
            docker_image,
            container_name: String::from("mimir-test-elasticsearch"),
            config,
        }
    }
}

impl DockerWrapper {
    pub fn new() -> DockerWrapper {
        DockerWrapper::default()
    }

    // Returns true if the container self.container_name is running
    // TODO Probably should run a check on Elasticsearch status
    pub async fn is_container_available(&mut self) -> Result<bool, Error> {
        let docker = Docker::connect_with_unix(
            "unix:///var/run/docker.sock",
            120,
            &bollard::ClientVersion {
                major_version: 1,
                minor_version: 24,
            },
        )
        .context(DockerConnection)?;

        let docker = &docker.negotiate_version().await.context(Version)?;

        docker.version().await.context(Version)?;

        let mut filters = HashMap::new();
        filters.insert("name", vec![self.container_name.as_str()]);

        let options = Some(ListContainersOptions {
            all: false, // only running containers
            filters,
            ..Default::default()
        });

        let containers = docker
            .list_containers(options)
            .await
            .context(DockerEngine)?;

        Ok(!containers.is_empty())
    }

    // If the container is already created, then start it.
    // If it is not created, then create it and start it.
    pub async fn create_container(&mut self) -> Result<(), Error> {
        let docker = Docker::connect_with_unix(
            "unix:///var/run/docker.sock",
            120,
            &bollard::ClientVersion {
                major_version: 1,
                minor_version: 24,
            },
        )
        .context(DockerConnection)?;

        let docker = docker.negotiate_version().await.context(Version)?;

        let _ = docker.version().await.context(Version);

        let mut filters = HashMap::new();
        filters.insert("name", vec![self.container_name.as_str()]);

        let options = Some(ListContainersOptions {
            all: true, // only running containers
            filters,
            ..Default::default()
        });

        let containers = docker
            .list_containers(options)
            .await
            .context(DockerEngine)?;

        if containers.is_empty() {
            let options = CreateContainerOptions {
                name: &self.container_name,
            };

            let mut port_bindings = HashMap::new();
            for (host_port, container_port) in self.ports.iter() {
                port_bindings.insert(
                    format!("{}/tcp", &container_port),
                    Some(vec![PortBinding {
                        host_ip: Some(String::from("0.0.0.0")),
                        host_port: Some(host_port.to_string()),
                    }]),
                );
            }

            let host_config = HostConfig {
                port_bindings: Some(port_bindings),
                memory: Some(1_000_000_000), // limit docker container to use 1GB of ram
                ..Default::default()
            };

            let mut exposed_ports = HashMap::new();
            self.ports.iter().for_each(|(_, container)| {
                let v: HashMap<(), ()> = HashMap::new();
                exposed_ports.insert(format!("{}/tcp", container), v);
            });

            let env_vars = vec![String::from("discovery.type=single-node")];

            let config = BollardConfig {
                image: Some(self.docker_image.clone()),
                exposed_ports: Some(exposed_ports),
                host_config: Some(host_config),
                env: Some(env_vars),
                ..Default::default()
            };

            docker
                .create_image(
                    Some(CreateImageOptions {
                        from_image: self.docker_image.clone(),
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .try_collect::<Vec<_>>()
                .await
                .context(DockerEngine)?;

            let _ = docker
                .create_container(Some(options), config)
                .await
                .context(DockerEngine)?;

            sleep(Duration::from_secs(5)).await;
        }
        let _ = docker
            .start_container(&self.container_name, None::<StartContainerOptions<String>>)
            .await
            .context(DockerEngine)?;

        sleep(Duration::from_secs(30)).await;

        Ok(())
    }

    /// This function cleans up the Elasticsearch
    async fn cleanup(&mut self) -> Result<(), Error> {
        let pool = remote::connection_test_pool()
            .await
            .context(ElasticsearchPoolCreation)?;
        let storage = pool
            .conn(Default::default())
            .await
            .context(ElasticsearchConnection)?;

        let _ = storage
            .client
            .indices()
            .delete(IndicesDeleteParts::Index(&["*"]))
            .request_timeout(storage.config.timeout)
            .send()
            .await
            .context(ElasticsearchClient)?;

        let _ = storage
            .client
            .indices()
            .delete_alias(IndicesDeleteAliasParts::IndexName(&["*"], &["*"]))
            .request_timeout(storage.config.timeout)
            .send()
            .await
            .context(ElasticsearchClient)?;

        let _ = storage
            .client
            .indices()
            .delete_index_template(IndicesDeleteIndexTemplateParts::Name("*"))
            .request_timeout(storage.config.timeout)
            .send()
            .await
            .context(ElasticsearchClient)?;

        sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    async fn _drop(&mut self) {
        if std::env::var("DONT_KILL_THE_WHALE") == Ok("1".to_string()) {
            println!(
                "the docker won't be stoped at the end, you can debug it.
                Note: ES has been mapped to the port 9242 in you localhost
                manually stop and rm the container mimirsbrunn_tests after debug"
            );
            return;
        }
        let docker = Docker::connect_with_unix(
            "unix:///var/run/docker.sock",
            120,
            &bollard::ClientVersion {
                major_version: 1,
                minor_version: 24,
            },
        )
        .expect("docker connection");

        let options = Some(bollard::container::StopContainerOptions { t: 0 });
        docker
            .stop_container(&self.container_name, options)
            .await
            .expect("stop container");

        let options = Some(bollard::container::RemoveContainerOptions {
            force: true,
            ..Default::default()
        });

        let _res = docker
            .remove_container(&self.container_name, options)
            .await
            .expect("remove container");
    }
}
