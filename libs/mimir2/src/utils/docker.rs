use bollard::container::{Config, CreateContainerOptions};
use bollard::errors::Error as BollardError;
use bollard::Docker;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::thread;
use tokio::runtime::Handle;
use tokio::time::{sleep, Duration};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Connection to docker socket: {}", source))]
    Connection { source: BollardError },

    #[snafu(display("docker version: {}", source))]
    Version { source: BollardError },
}

pub struct DockerWrapper {
    ports: Vec<Vec<u32>>, // FIXME Should HashMap
    docker_image: String,
    container_name: String, // ip: String,
}

impl DockerWrapper {
    // pub fn host(&self) -> String {
    //     format!("http://{}:9200", self.ip)
    // }

    async fn setup(&mut self) -> Result<(), Error> {
        println!("Establishing docker connection");
        let docker = Docker::connect_with_unix(
            "unix:///var/run/docker.sock",
            120,
            &bollard::ClientVersion {
                major_version: 1,
                minor_version: 24,
            },
        )
        .context(Connection)?;

        let docker = &docker.negotiate_version().await.context(Version)?;

        &docker.version().await.context(Version);

        let options = CreateContainerOptions {
            name: &self.container_name,
        };

        let mut exposed_ports = HashMap::new();
        self.ports.iter().for_each(|ps| {
            let v: HashMap<(), ()> = HashMap::new();
            exposed_ports.insert(format!("{}/tcp", ps[0]), v);
        });

        let config = Config {
            image: Some(String::from(self.docker_image.clone())),
            exposed_ports: Some(exposed_ports),
            ..Default::default()
        };

        let create_resp = &docker.create_container(Some(options), config).await;

        assert!(create_resp.is_ok());

        println!("Waiting 10s");
        sleep(Duration::from_millis(10000)).await;

        Ok(())
    }

    pub async fn new() -> Result<DockerWrapper, Error> {
        let mut wrapper = DockerWrapper {
            ports: vec![vec![9200, 9200], vec![9300, 9300]],
            docker_image: String::from("docker.elastic.co/elasticsearch/elasticsearch:7.13.0"),
            container_name: String::from("mimir-test-elasticsearch"),
        };
        wrapper.setup().await?;
        // let rubber = Rubber::new_with_timeout(&wrapper.host(), Duration::from_secs(10)); // use a long timeout
        // rubber.initialize_templates().unwrap();
        Ok(wrapper)
    }
}

impl Drop for DockerWrapper {
    fn drop(&mut self) {
        // Inside an async block or function.
        println!("Inside drop");
        let container_name = self.container_name.clone();
        let handle = Handle::current();
        thread::spawn(move || {
            handle.spawn(async move {
                println!("Inside drop thread");
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

                println!("about to stop");
                //let options = Some(bollard::container::StopContainerOptions { t: 0 });
                // docker
                //     .stop_container(&container_name, options)
                //     .await
                //     .expect("stop container");
                println!("about to remove");
                let options = Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                });

                let _res = docker
                    .remove_container(&container_name, options)
                    .await
                    .expect("remove container");
                println!("done");
            });
        });
    }
}
