use clap::{App, Arg, SubCommand};
use snafu::{ResultExt, Snafu};

mod server;
mod settings;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Command Line Interface Error: {}", msg))]
    CLIError { msg: String },
    #[snafu(display("Server Error: {}", source))]
    ServerError {
        #[snafu(backtrace)]
        source: server::Error,
    },
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = App::new("Microservice for bragi")
        .version(VERSION)
        .author("Matthieu Paindavoine")
        .arg(
            Arg::with_name("config dir")
                .value_name("DIR")
                .short("c")
                .long("config-dir")
                .help("Config directory"),
        )
        .arg(
            Arg::with_name("settings")
                .value_name("STRING")
                .short("s")
                .long("settings")
                .help("Settings"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("publish bragi service")
                .version("0.1")
                .author("Matthieu Paindavoine <matt@area403.org>")
                .arg(
                    Arg::with_name("address")
                        .value_name("HOST")
                        .short("h")
                        .long("host")
                        .help("Address serving this server"),
                )
                .arg(
                    Arg::with_name("port")
                        .value_name("PORT")
                        .short("p")
                        .long("port")
                        .help("Port"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(_)) => server::run(&matches).await.context(ServerError),
        _ => Err(Error::CLIError {
            msg: String::from("Unrecognized subcommand"),
        }),
    }
}
