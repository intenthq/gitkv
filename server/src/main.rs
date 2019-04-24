#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;
#[macro_use]
extern crate log;
extern crate env_logger;

use actix_web::actix::{Actor, Addr, System};
use actix_web::{error, http, middleware, server, App, AsyncResponder, Binary, FromRequest, HttpRequest, Responder};
use env_logger::Env;
use futures::future;
use futures::future::Future;
use handlers::{CatFile, GitRepos};
use std::path::Path;

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

    let repos = git::load_repos(&repo_root);

    info!("Loaded Git repos: {:?}", repos.keys());

    let addr = GitRepos::new(repos).start();
    let listen_address = format!("{}:{}", host, port);

    info!("Listening on {}", listen_address);

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

fn extract_params(req: &HttpRequest<AppState>) -> Result<(actix_web::Path<PathParams>, actix_web::Query<QueryParams>), error::Error> {
    let path_params: actix_web::Path<PathParams> = actix_web::Path::<PathParams>::extract(req)?;
    let query_params: actix_web::Query<QueryParams> = actix_web::Query::<QueryParams>::extract(req)?;

    Ok((path_params, query_params))
}

fn get_repo(req: &HttpRequest<AppState>) -> impl Responder {
    let addr: Addr<GitRepos> = req.state().git_repos.clone();
    let params_fut = future::result(extract_params(req));

    params_fut.and_then(move |(path_params, query_params)| {
        let repo_key = path_params.repo.clone();
        let filename = query_params.file.clone();
        let reference = query_params
            .reference
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_REFERENCE)
            .to_string();
        ;
        // TODO return proper content type depending on the content of the blob
        addr.send(CatFile {
            repo_key,
            filename,
            reference,
        })
        .map_err(|e| error::InternalError::new(e, http::StatusCode::NOT_FOUND).into())
        .and_then(|x| {
            x.0.map(Binary::from)
                .map_err(|e| error::InternalError::new(e, http::StatusCode::NOT_FOUND).into())
        })
    }).responder()
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
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};
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

    fn start_test_server() -> TestServer {
        TestServer::build_with_state(|| {
            let addr = GitRepos::new(git::load_repos(Path::new("../../"))).start();
            AppState {
                git_repos: addr.clone(),
            }
        })
        .start(|app| {
            app.resource("/repo/{repo}", |r| r.h(&super::get_repo));
        })
    }

    macro_rules! assert_test_server_responds_with {
        ($path:expr, $expected_status:expr, $expected_body:expr) => {
            {
                let mut srv = start_test_server();

                let request = srv.client(http::Method::GET, &$path).finish().unwrap();
                let response = srv.execute(request.send()).unwrap();
                let bytes = srv.execute(response.body()).unwrap();
                let body = str::from_utf8(&bytes).unwrap();

                assert_eq!(response.status(), $expected_status);
                assert_eq!(body, $expected_body);
            }
        };
    }

    #[test]
    fn get_repo_with_missing_name() {
        assert_test_server_responds_with!(
            "/repo/idontexist?file=README.md&reference=origin/master",
            404,
            ""
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
            "/repo/gitkv?file=not-a-file&reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn get_repo_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repo/gitkv?file=server/resources/test-file&reference=idonot/exist",
            404,
            ""
        )
    }

    #[test]
    fn get_repo_with_missing_path_parameter() {
        assert_test_server_responds_with!(
            "/repo/gitkv?reference=origin/master",
            400,
            ""
        )
    }

    #[test]
    fn get_repo_with_valid_file() {
        assert_test_server_responds_with!(
            "/repo/gitkv?file=Cargo.toml",
            200,
            "[workspace]\n\nmembers = [\n    \"git\",\n    \"handlers\",\n    \"server\",\n]\n"
        );
    }

    #[test]
    fn get_repo_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            // This reference is the very first commit in this repo, where the
            // README.md was still pretty much empty.
            "/repo/gitkv?file=README.md&reference=079b0a3afe57bdf9e428e5dbf3919adaff905ffe",
            200,
            "# gitkv\ngitkv is a server for using git as a key value store for text files\n"
        );
    }

    #[test]
    fn get_repo_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repo/gitkv?file=Cargo.toml&reference=v0.0.1",
            200,
            "[workspace]\n\nmembers = [\n    \"git\",\n    \"handlers\",\n    \"server\",\n]\n"
        );
    }
}
