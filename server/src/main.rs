#[macro_use]
extern crate clap;
extern crate actix_web;
extern crate listenfd;

use actix_web::{server, App};
use listenfd::ListenFd;
use handlers::RepoHandler;

const DEFAULT_PORT: &str = "7791";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_REPO_PATH: &str = "./";

#[derive(Debug)]
enum AppError {
    GitError(git2::Error),
    ObjectNotFound(String),
}

impl From<git2::Error> for AppError {
    fn from(error: git2::Error) -> Self {
        match error.code() {
            git2::ErrorCode::NotFound => AppError::ObjectNotFound(error.to_string()),
            _ => AppError::GitError(error),
        }
    }
}

fn main() {
    let args = parse_args().get_matches();

    let host = args.value_of("host").unwrap_or(DEFAULT_HOST);
    let port = args.value_of("port").unwrap_or(DEFAULT_PORT);

    create_server(host, port).run();
}

fn create_server(host: &str, port: &str) -> server::HttpServer<App<()>, fn() -> App<()>> {

    let mut listenfd = ListenFd::from_env();
    let server: server::HttpServer<App<()>, fn() -> App<()>> = server::new(|| {
        App::new()
            .resource("/repo/{repo}", |r| r.h(RepoHandler::new(DEFAULT_REPO_PATH.to_string())))
    });

    match listenfd.take_tcp_listener(0).expect("can't take tcp listener") {
        Some(l) => server.listen(l),
        None => {
            let address = format!("{}:{}", host, port);
            println!("Listening to {}", address);
            server.bind(address).expect("can't bind into address")
        }
    }
}

fn parse_args<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .arg(
            clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .value_name("PORT")
                .default_value(DEFAULT_PORT)
                .help("port to listen to"),
        )
        .arg(
            clap::Arg::with_name("host")
                .short("h")
                .long("host")
                .takes_value(true)
                .value_name("HOST")
                .default_value(DEFAULT_HOST)
                .help("host to listen to"),
        )
}
