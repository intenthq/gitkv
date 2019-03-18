#[macro_use]
extern crate serde_derive;

use actix_web::{dev::Handler, Binary, FromRequest, HttpRequest, Path, Query};
use git::{GitHelper, GitOps};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct PathParams {
    repo: String,
}

#[derive(Deserialize)]
pub struct QueryParams {
    reference: String,
    file: String,
}

pub struct RepoHandler {
    pub repo_root: String,
    git_ops: Box<GitOps>,
}

impl<S> Handler<S> for RepoHandler {
    type Result = Binary;

    fn handle(&self, req: &HttpRequest<S>) -> Self::Result {
        //TODO https://actix.rs/docs/errors/
        let path_params = Path::<PathParams>::extract(req).expect("Wrong path params");
        let query_params = Query::<QueryParams>::extract(req).expect("Wront query params");
        let repo_path: PathBuf = [&self.repo_root, &path_params.repo].iter().collect();
        let reference = format!("refs/{}", query_params.reference);
        //TODO return proper content type depending on the content of the blob
        self.git_ops
            .cat_file(&repo_path, &reference, &query_params.file)
            .map(Binary::from)
            .expect("Can't cat file")
    }
}

impl RepoHandler {
    pub fn new(repo_root: String) -> RepoHandler {
        RepoHandler {
            repo_root,
            git_ops: Box::new(GitHelper {}),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::RepoHandler;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::{Binary, Body};
    use git::GitOps;
    use std::path;

    struct TestGitOps {
        res: Vec<u8>,
    }

    impl GitOps for TestGitOps {
        fn cat_file(
            &self,
            _repo_path: &path::Path,
            _reference: &str,
            _filename: &str,
        ) -> Result<Vec<u8>, git2::Error> {
            Ok(self.res.to_owned())
        }
    }

    fn bin_ref(body: &Body) -> &Binary {
        match *body {
            Body::Binary(ref bin) => bin,
            _ => panic!(),
        }
    }

    #[test]
    fn it_returns_the_content_of_the_file_by_cat_file() {
        let rp = RepoHandler {
            repo_root: "not-used".to_string(),
            git_ops: Box::new(TestGitOps {
                res: b"hello".to_vec(),
            }),
        };

        let resp = test::TestRequest::with_header("content-type", "application/json")
            .param("repo", "client-config.git")
            .uri("/repo/?reference=the-reference&file=the-file")
            .run(&rp)
            .expect("can't run test request");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(bin_ref(resp.body()), &Binary::from_slice(b"hello"));
    }

}
