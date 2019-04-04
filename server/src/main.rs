#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;

use actix_web::actix::{Actor, Addr, System};
use actix_web::{http, middleware, server, App, Binary, FromRequest, HttpRequest, Responder};
use futures::future::Future;
use handlers::{CatFile, GitRepos};
use std::path::Path;

const DEFAULT_PORT: &str = "7791";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_REPO_ROOT: &str = "./";
const DEFAULT_REFERENCE: &str = "heads/master";

fn main() {
    let args = parse_args().get_matches();

    let host = args.value_of("host").unwrap_or(DEFAULT_HOST);
    let port = args.value_of("port").unwrap_or(DEFAULT_PORT);
    let repo_root = Path::new(args.value_of("repo-root").unwrap_or(DEFAULT_REPO_ROOT));

    run_server(host, port, repo_root);
}

#[derive(Deserialize)]
pub struct PathParams {
    pub repo: String,
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub reference: Option<String>,
    pub file: String,
}

pub struct AppState {
    pub git_repos: Addr<GitRepos>,
}

fn run_server(host: &str, port: &str, repo_root: &Path) {
    let _sys = System::new("gitkv-server");

    let addr = GitRepos::new(git::load_repos(&repo_root).expect("can't load repos")).start();

    let listen_address = format!("{}:{}", host, port);

    server::new(move || {
        App::with_state(AppState {
            git_repos: addr.clone(),
        })
        .middleware(middleware::Logger::default())
        .resource("/repo/{repo}", |r| r.method(http::Method::GET).f(get_repo))
    })
    .bind(listen_address)
    .expect("can't bind into address")
    .run();
}

fn get_repo(req: &HttpRequest<AppState>) -> impl Responder {
    //TODO https://actix.rs/docs/errors/
    let path_params = actix_web::Path::<PathParams>::extract(req).expect("Wrong path params");
    let query_params = actix_web::Query::<QueryParams>::extract(req).expect("Wrong query params");
    let repo_key = path_params.repo.to_string();
    let filename = query_params.file.to_string();
    let reference = format!(
        "refs/{}",
        query_params
            .reference
            .as_ref()
            .unwrap_or(&DEFAULT_REFERENCE.to_string())
    );
    let gr: &Addr<GitRepos> = &req.state().git_repos;
    //TODO return proper content type depending on the content of the blob
    gr.send(CatFile {
            repo_key,
            filename,
            reference,
        })
        .map(|x| {
            x.0.map(Binary::from)
                .map_err(|e| actix_web::error::InternalError::new(e, http::StatusCode::NOT_FOUND))
        })
        //TODO don't wait and return the future itself
        .wait()
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
        .arg(
            clap::Arg::with_name("repo-root")
                .short("r")
                .long("repo-root")
                .takes_value(true)
                .value_name("PATH")
                .default_value(DEFAULT_REPO_ROOT)
                .help("path where the different repositories are located"),
        )
}
