#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate job_scheduler;

use job_scheduler::{JobScheduler, Job};
use actix::{Actor, Addr, System};
use actix_web::{error, http, middleware, web, App, HttpServer};
use env_logger::Env;
use futures::future::Future;
use handlers::{CatFile, GitRepos};
use std::path::Path;
use std::time::Duration;
use std::thread;

const DEFAULT_PORT: &str = "7791";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_REPO_ROOT: &str = "./";
const DEFAULT_REFERENCE: &str = "origin/master";

fn main() {
    env_logger::from_env(Env::default().default_filter_or("gitkv=info")).init();

    let args = parse_args().get_matches();

    let host = args.value_of("host").unwrap_or(DEFAULT_HOST);
    let port = args.value_of("port").unwrap_or(DEFAULT_PORT);
    let repo_root = Path::new(args.value_of("repo-root").unwrap_or(DEFAULT_REPO_ROOT));

    thread::spawn(move || {
        refresh()
    });

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

fn refresh() {
    let mut sched = JobScheduler::new();

    sched.add(Job::new("1/10 * * * * *".parse().unwrap(), || {

    }));

    loop {
        sched.tick();

        std::thread::sleep(Duration::from_millis(500));
    }

}

fn run_server(host: &str, port: &str, repo_root: &Path){
    let _sys = System::new("gitkv-server");

    let repos = git::load_repos(&repo_root);

    info!("Loaded Git repos: {:?}", repos.keys());

    let addr = GitRepos::new(repos).start();
    let listen_address = format!("{}:{}", host, port);

    info!("Listening on {}", listen_address);

    HttpServer::new(move || {
        App::new().data(AppState {
            git_repos: addr.clone(),
        })
        .wrap(middleware::Logger::default())
        .route("/repo/{repo}", web::get().to_async(get_repo))
    })
    .bind(listen_address)
    .expect("can't bind into address")
    .run()
    .expect("could not start server");
}

fn get_repo((app_state, path_params, query_params): (web::Data<AppState>, web::Path<PathParams>, web::Query<QueryParams>))
    -> impl Future<Item=web::Bytes, Error=error::Error> {
        let addr: Addr<GitRepos> = app_state.git_repos.clone();
        let repo_key = path_params.repo.clone();
        let filename = query_params.file.clone();
        let reference = query_params
            .reference
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_REFERENCE)
            .to_string();

        // TODO return proper content type depending on the content of the blob
        addr.send(CatFile {
            repo_key,
            filename,
            reference,
        })
        .map_err(|e| error::InternalError::new(e, http::StatusCode::NOT_FOUND).into())
        .and_then(|x| {
            x.0.map(web::Bytes::from)
                .map_err(|e| error::InternalError::new(e, http::StatusCode::NOT_FOUND).into())
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_http::h1;
    use actix_http_test::{TestServer, TestServerRuntime};
    use std::str;

    #[test]
    #[should_panic]
    fn run_server_with_invalid_host() {
        run_server("", DEFAULT_PORT, Path::new(DEFAULT_REPO_ROOT));
    }

    #[test]
    #[should_panic]
    fn run_server_with_invalid_port() {
        run_server(DEFAULT_HOST, "", Path::new(DEFAULT_REPO_ROOT));
    }

    #[test]
    #[should_panic]
    fn run_server_with_invalid_repo() {
        run_server(DEFAULT_HOST, DEFAULT_PORT, Path::new(""));
    }

    fn start_test_server() -> TestServerRuntime {
        TestServer::new(|| {
            let addr = GitRepos::new(git::load_repos(Path::new("test"))).start();

            h1::H1Service::new(
                App::new().data(AppState {
                    git_repos: addr.clone(),
                })
                .wrap(middleware::Logger::default())
                .route("/repo/{repo}", web::get().to_async(get_repo))
            )
        })
    }

    macro_rules! assert_test_server_responds_with {
        ($path:expr, $expected_status:expr, $expected_body:expr) => {{
            let mut srv = start_test_server();

            let mut response = srv.block_on(srv.get(&$path).send()).unwrap();
            let bytes = srv.block_on(response.body()).unwrap();
            let body = str::from_utf8(&bytes).unwrap();

            assert_eq!(response.status(), $expected_status);
            assert_eq!(body, $expected_body);
        }};
    }

    #[test]
    fn get_repo_with_missing_name() {
        assert_test_server_responds_with!(
            "/repo/idontexist?file=README.md&reference=origin/master",
            404,
            "No repo found with name 'idontexist'"
        )
    }

    #[test]
    fn get_repo_with_empty_name() {
        assert_test_server_responds_with!(
            "/repo/?file=README.md&reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn get_repo_with_invalid_file() {
        assert_test_server_responds_with!(
            "/repo/fixtures?file=not-a-file&reference=origin/master",
            404,
            "the path 'not-a-file' does not exist in the given tree; class=Tree (14); code=NotFound (-3)"
        )
    }

    #[test]
    fn get_repo_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repo/fixtures?file=example.txt&reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }

    #[test]
    fn get_repo_with_missing_path_parameter() {
        assert_test_server_responds_with!(
            "/repo/fixtures?reference=origin/master",
            400,
            "Query deserialize error: missing field `file`"
        )
    }

    #[test]
    fn get_repo_with_valid_file() {
        assert_test_server_responds_with!(
            "/repo/fixtures?file=example.txt",
            200,
            "Bux poi\n"
        );
    }

    #[test]
    fn get_repo_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            "/repo/fixtures?file=example.txt&reference=167c95c4c023c6a79c6efa15fc5adadbf04aaf81",
            200,
            "Foo bar\n"
        );
    }

    #[test]
    fn get_repo_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repo/fixtures?file=example.txt&reference=v0.1",
            200,
            "Release 1.0 file contents\n"
        );
    }
}
