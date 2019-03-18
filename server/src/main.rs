#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;
extern crate git2;
extern crate listenfd;

use actix_web::{http, server, App, Path, Query, Responder, Binary};
use git2::Repository;
use listenfd::ListenFd;
use git;

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

#[derive(Deserialize)]
struct PathParams {
    repo: String,
}

#[derive(Deserialize)]
struct QueryParams {
    reference: String,
    file: String,
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
        App::new().resource("/repo/{repo}", |r| {
            r.method(http::Method::GET).with(repo_handler)
        })
    });

    match listenfd.take_tcp_listener(0).unwrap() {
        Some(l) => server.listen(l),
        None => {
            let address = format!("{}:{}", host, port);
            println!("Listening to {}", address);
            server.bind(address).unwrap()
        }
    }
}

fn repo_handler(path_params: Path<PathParams>, query_params: Query<QueryParams>) -> impl Responder {
    let repo_path = format!("{}{}", DEFAULT_REPO_PATH, path_params.repo);
    let reference = format!("refs/{}", query_params.reference);
    let repo = Repository::open(repo_path).unwrap();
    let f = git::cat_file(&repo, &reference, &query_params.file).unwrap();
    //TODO https://actix.rs/docs/errors/
    //TODO return proper content type depending on the content of the blob
    Binary::from(f)
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
