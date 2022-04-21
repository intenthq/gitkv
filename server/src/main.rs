#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate actix_web;
#[macro_use]
extern crate log;
extern crate env_logger;

use actix::{Actor, Addr};
use actix_web::{error, get, http, middleware, web, App, HttpServer};
use env_logger::Env;
use handlers::{
    CatFile, CatFileResponse, GitRepos, LsDir, LsDirResponse, ResolveRef, ResolveRefResponse,
};
use std::path::{Path, PathBuf};

const DEFAULT_PORT: &str = "7791";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_REPO_ROOT: &str = "./";
const DEFAULT_REFERENCE: &str = "origin/master";

#[derive(Deserialize)]
pub struct PathParams {
    pub repo: String,
    pub path: PathBuf,
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

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("gitkv=info")).init();

    let args = parse_args().get_matches();

    let host = args.value_of("host").unwrap_or(DEFAULT_HOST);
    let port = args.value_of("port").unwrap_or(DEFAULT_PORT);
    let repo_root = Path::new(args.value_of("repo-root").unwrap_or(DEFAULT_REPO_ROOT));

    match run_server(host, port, repo_root).await {
        Ok(_) => {
            info!("gitkv stopped gracefully");
            std::process::exit(0);
        }
        Err(e) => {
            error!("gitkv stopped due to an error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_server(host: &str, port: &str, repo_root: &Path) -> std::io::Result<()> {
    let repos = git::load_repos(&repo_root);

    info!("Loaded Git repos: {:?}", repos.keys());

    let addr = GitRepos::new(repos).start();
    let listen_address = format!("{}:{}", host, port);

    info!("Listening on {}", listen_address);

    HttpServer::new(move || {
        App::new()
            .data(AppState {
                git_repos: addr.clone(),
            })
            .wrap(middleware::Logger::default())
            .service(cat_file)
            .service(ls_dir)
            .service(resolve_ref)
    })
    .bind(listen_address)?
    .run()
    .await
}

macro_rules! not_found {
    () => {
        |err| error::InternalError::new(err, http::StatusCode::NOT_FOUND).into()
    };
}

#[get("/repos/{repo}/cat/{path:.+}")]
async fn cat_file(
    (app_state, path_params, query_params): (
        web::Data<AppState>,
        web::Path<PathParams>,
        web::Query<QueryParams>,
    ),
) -> Result<web::Bytes, error::Error> {
    let addr: Addr<GitRepos> = app_state.git_repos.clone();
    let repo_key = path_params.repo.clone();
    let path = path_params.path.clone();
    let reference = query_params
        .reference
        .as_deref()
        .unwrap_or(DEFAULT_REFERENCE)
        .to_string();

    // TODO return proper content type depending on the content of the blob
    addr.send(CatFile {
        repo_key,
        reference,
        path,
    })
    .await
    .map_err(not_found!())
    .and_then(|CatFileResponse(resp)| resp.map(web::Bytes::from).map_err(not_found!()))
}

#[get("/repos/{repo}/ls/{path:.+}")]
async fn ls_dir(
    (app_state, path_params, query_params): (
        web::Data<AppState>,
        web::Path<PathParams>,
        web::Query<QueryParams>,
    ),
) -> Result<String, error::Error> {
    let addr: Addr<GitRepos> = app_state.git_repos.clone();
    let repo_key = path_params.repo.clone();
    let path = path_params.path.clone();
    let reference = query_params
        .reference
        .as_deref()
        .unwrap_or(DEFAULT_REFERENCE)
        .to_string();

    addr.send(LsDir {
        repo_key,
        reference,
        path,
    })
    .await
    .map_err(not_found!())
    .and_then(|LsDirResponse(resp)| {
        resp.map_err(not_found!())
            .and_then(|children| serde_json::to_string(&children).map_err(not_found!()))
    })
}

#[get("/repos/{repo}/resolve")]
async fn resolve_ref(
    (app_state, repo_path_params, query_params): (
        web::Data<AppState>,
        web::Path<RepoPathParams>,
        web::Query<QueryParams>,
    ),
) -> Result<String, error::Error> {
    let addr: Addr<GitRepos> = app_state.git_repos.clone();
    let repo_key = repo_path_params.repo.clone();
    let reference = query_params
        .reference
        .as_deref()
        .unwrap_or(DEFAULT_REFERENCE)
        .to_string();

    addr.send(ResolveRef {
        repo_key,
        reference,
    })
    .await
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
    use actix_web::{test, App};
    use std::str;

    fn start_test_server() -> test::TestServer {
        test::start_with(test::config().h1(), || {
            let addr = GitRepos::new(git::load_repos(Path::new("test"))).start();

            App::new()
                .data(AppState { git_repos: addr })
                .service(cat_file)
                .service(ls_dir)
                .service(resolve_ref)
        })
    }

    macro_rules! assert_test_server_responds_with {
        ($path:expr, $expected_status:expr, $expected_body:expr) => {{
            let srv = start_test_server();

            let req = srv.get(&$path);
            let mut resp = req.send().await.unwrap();
            let bytes = resp.body().await.unwrap();
            let body = str::from_utf8(&bytes).unwrap();

            assert_eq!(resp.status(), $expected_status);
            assert_eq!(body, $expected_body);
        }};
    }

    // cat tests

    #[actix_rt::test]
    async fn cat_file_with_empty_repo() {
        assert_test_server_responds_with!("/repos//cat/README.md?reference=origin/master", 404, "")
    }

    #[actix_rt::test]
    async fn cat_file_with_empty_path() {
        assert_test_server_responds_with!("/repos/fixtures/cat/?reference=origin/master", 404, "")
    }

    #[actix_rt::test]
    async fn cat_file_with_invalid_repo() {
        assert_test_server_responds_with!(
            "/repos/idontexist/cat/README.md?reference=origin/master",
            404,
            "No repo found with name 'idontexist'"
        )
    }

    #[actix_rt::test]
    async fn cat_file_with_invalid_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/not-a-file?reference=origin/master",
            404,
            "the path 'not-a-file' does not exist in the given tree; class=Tree (14); code=NotFound (-3)"
        )
    }

    #[actix_rt::test]
    async fn cat_file_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }

    #[actix_rt::test]
    async fn cat_file_with_valid_file() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt",
            200,
            "Bux poi — updated!\n"
        );
    }

    #[actix_rt::test]
    async fn cat_file_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "Bux poi\n"
        );
    }

    #[actix_rt::test]
    async fn cat_file_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/cat/example.txt?reference=v0.1.0",
            200,
            "Bux poi\n"
        );
    }

    // ls tests

    #[actix_rt::test]
    async fn ls_dir_with_empty_repo() {
        assert_test_server_responds_with!("/repos//ls/a-dir?reference=origin/master", 404, "")
    }

    #[actix_rt::test]
    async fn ls_dir_with_empty_path() {
        assert_test_server_responds_with!("/repos/fixtures/ls/?reference=origin/master", 404, "")
    }

    #[actix_rt::test]
    async fn ls_dir_with_invalid_repo() {
        assert_test_server_responds_with!(
            "/repos/idontexist/ls/a-dir?reference=origin/master",
            404,
            "No repo found with name 'idontexist'"
        )
    }

    #[actix_rt::test]
    async fn ls_dir_with_invalid_path() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/not-a-dir?reference=origin/master",
            404,
            "the path 'not-a-dir' does not exist in the given tree; class=Tree (14); code=NotFound (-3)"
        )
    }

    #[actix_rt::test]
    async fn ls_dir_with_invalid_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/example.txt?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }

    #[actix_rt::test]
    async fn ls_dir_with_valid_dir() {
        // Note that we do not expect recursive results — so `a-dir/nested-dir`
        // and its children are expected to be absent!
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"file-d\",\"nested-dir\"]"
        );
    }

    #[actix_rt::test]
    async fn ls_dir_with_valid_sha_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"nested-dir\"]"
        );
    }

    #[actix_rt::test]
    async fn ls_dir_with_valid_tag_reference_parameter() {
        assert_test_server_responds_with!(
            "/repos/fixtures/ls/a-dir?reference=v0.1.0",
            200,
            "[\"file-a\",\"file-b\",\"file-c\",\"nested-dir\"]"
        );
    }

    // resolve tests

    fn origin_master_sha() -> String {
        String::from("e6134971608eb6ba7eb29047d5884c3377bc1fd2")
    }

    #[actix_rt::test]
    async fn resolve_ref_with_empty_reference() {
        assert_test_server_responds_with!("/repos/fixtures/resolve", 200, origin_master_sha())
    }

    #[actix_rt::test]
    async fn resolve_ref_with_branch_name() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=origin/master",
            200,
            origin_master_sha()
        )
    }

    #[actix_rt::test]
    async fn resolve_ref_with_commit_sha() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=467e981f94686d7a1db395f8acfd3cf7e7adfcd3",
            200,
            "467e981f94686d7a1db395f8acfd3cf7e7adfcd3"
        )
    }

    #[actix_rt::test]
    async fn resolve_ref_with_tag() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=v0.1.0",
            200,
            "467e981f94686d7a1db395f8acfd3cf7e7adfcd3"
        )
    }

    #[actix_rt::test]
    async fn resolve_ref_with_invalid_ref() {
        assert_test_server_responds_with!(
            "/repos/fixtures/resolve?reference=idonot/exist",
            404,
            "revspec 'idonot/exist' not found; class=Reference (4); code=NotFound (-3)"
        )
    }
}
