#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;
#[macro_use]
extern crate log;
extern crate env_logger;

use actix::{Actor, Addr, System};
use actix_web::{error, http, middleware, web, App, HttpServer};
use env_logger::Env;
use futures::future::Future;
use handlers::{CatFile, CatFileResponse, GitRepos, LsDir, LsDirResponse, ResolveRef, ResolveRefResponse,};
use std::path::{Path, PathBuf};
use serde_json;

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
    pub path: PathBuf
}

#[derive(Deserialize)]
pub struct RepoPathParams {
    pub repo: String,
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub reference: Option<String>,
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

    HttpServer::new(move || {
        App::new().data(AppState {
            git_repos: addr.clone(),
        })
        .wrap(middleware::Logger::default())
        .route("/repos/{repo}/cat/{path:.+}", web::get().to_async(cat_file))
        .route("/repos/{repo}/ls/{path:.+}", web::get().to_async(ls_dir))
        .route("/repos/{repo}/resolve", web::get().to_async(resolve_ref))
    })
    .bind(listen_address)
    .expect("can't bind into address")
    .run()
    .expect("could not start server");
}

macro_rules! not_found {
    () => {
        |err| error::InternalError::new(err, http::StatusCode::NOT_FOUND).into()
    };
}

fn cat_file((app_state, path_params, query_params): (web::Data<AppState>, web::Path<PathParams>, web::Query<QueryParams>))
    -> impl Future<Item=web::Bytes, Error=error::Error> {
        let addr: Addr<GitRepos> = app_state.git_repos.clone();
        let repo_key = path_params.repo.clone();
        let path = path_params.path.clone();
        let reference = query_params
            .reference
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_REFERENCE)
            .to_string();

        // TODO return proper content type depending on the content of the blob
        addr.send(CatFile {
            repo_key,
            path,
            reference,
        })
        .map_err(not_found!())
        .and_then(|CatFileResponse(resp)| resp.map(web::Bytes::from).map_err(not_found!()))
    }

fn ls_dir((app_state, path_params, query_params): (web::Data<AppState>, web::Path<PathParams>, web::Query<QueryParams>))
    -> impl Future<Item=String, Error=error::Error> {
        let addr: Addr<GitRepos> = app_state.git_repos.clone();
        let repo_key = path_params.repo.clone();
        let path = path_params.path.clone();
        let reference = query_params
            .reference
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_REFERENCE)
            .to_string();

        addr.send(LsDir {
            repo_key,
            path,
            reference,
        })
        .map_err(not_found!())
        .and_then(|LsDirResponse(resp)| {
            resp.map_err(not_found!()).and_then(|children| {
                serde_json::to_string(&children).map_err(not_found!())
            })
        })
    }
fn resolve_ref(
    (app_state, repo_path_params, query_params): (
        web::Data<AppState>,
        web::Path<RepoPathParams>,
        web::Query<QueryParams>,
    ),
) -> impl Future<Item = String, Error = error::Error> {
    let addr: Addr<GitRepos> = app_state.git_repos.clone();
    let repo_key = repo_path_params.repo.clone();
    let reference = query_params
        .reference
        .as_ref()
        .map(String::as_str)
        .unwrap_or(DEFAULT_REFERENCE)
        .to_string();
    addr.send(ResolveRef {
        repo_key,
        reference,
    })
    .map_err(not_found!())
    .and_then(|ResolveRefResponse(resp)| resp.map_err(not_found!()))
}

fn parse_args<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new(crate_name!())
        .version(crate_version!())
        // FIXME: Switch back to `crate_authors` macro once deprecation warnings are fixed in
        // stable warnings.
        //
        // .author(crate_authors!("\n"))
        .author("Intent HQ")
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
                    git_repos: addr,
                })
                .wrap(middleware::Logger::default())
                .route("/repos/{repo}/cat/{path:.+}", web::get().to_async(cat_file))
                .route("/repos/{repo}/ls/{path:.+}", web::get().to_async(ls_dir))
                    .route("/repos/{repo}/resolve", web::get().to_async(resolve_ref)),
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

    // cat tests

    #[test]
    fn cat_file_with_empty_repo() {
        assert_test_server_responds_with!(
            "/repos//cat/README.md?reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn cat_file_with_empty_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/?reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn cat_file_with_invalid_repo() {
        assert_test_server_responds_with!(
            "/repos/idontexist/cat/README.md?reference=origin/master",
            404,
            "No repo found with name 'idontexist'"
        )
    }

    #[test]
    fn cat_file_with_invalid_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/not-a-file?reference=origin/master",
            404,
            "the path 'not-a-file' does not exist in the given tree; class=Tree (14); code=NotFound (-3)"
        )
    }

    #[test]
    fn cat_file_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }

    #[test]
    fn cat_file_with_valid_file() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt",
            200,
            "Bux poi — updated!\n"
        );
    }

    #[test]
    fn cat_file_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "Bux poi\n"
        );
    }

    #[test]
    fn cat_file_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=v0.1.0",
            200,
            "Bux poi\n"
        );
    }

    // ls tests

    #[test]
    fn ls_dir_with_empty_repo() {
        assert_test_server_responds_with!(
            "/repos//ls/a-dir?reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn ls_dir_with_empty_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/?reference=origin/master",
            404,
            ""
        )
    }

    #[test]
    fn ls_dir_with_invalid_repo() {
        assert_test_server_responds_with!(
            "/repos/idontexist/ls/a-dir?reference=origin/master",
            404,
            "No repo found with name 'idontexist'"
        )
    }

    #[test]
    fn ls_dir_with_invalid_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/not-a-dir?reference=origin/master",
            404,
            "the path 'not-a-dir' does not exist in the given tree; class=Tree (14); code=NotFound (-3)"
        )
    }

    #[test]
    fn ls_dir_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/example.txt?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }

    #[test]
    fn ls_dir_with_valid_dir() {
        // Note that we do not expect recursive results — so `a-dir/nested-dir`
        // and its children are expected to be absent!
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"file-d\",\"nested-dir\"]"
        );
    }

    #[test]
    fn ls_dir_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"nested-dir\"]"
        );
    }

    #[test]
    fn ls_dir_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir?reference=v0.1.0",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"nested-dir\"]"
        );
    }

    // resolve tests

    #[test]
    fn resolve_ref_with_empty_repo() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "467e981f94686d7a1db395f8acfd3cf7e7adfcd3"
        )
    }

    #[test]
    fn resolve_ref_with_invalid_ref() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }
}
