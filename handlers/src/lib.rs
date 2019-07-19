use actix::dev::{MessageResponse, ResponseChannel};
use actix::{Actor, Context, Handler, Message};
use git::{git2::Repository, GitOps, LibGitOps};
use std::collections::HashMap;

pub struct CatFile {
    pub repo_key: String,
    pub reference: String,
    pub path: String,
}

impl Message for CatFile {
    type Result = CatFileResponse;
}

pub struct CatFileResponse(pub Result<Vec<u8>, String>);

impl<A, M> MessageResponse<A, M> for CatFileResponse
    where A: Actor, M: Message<Result = CatFileResponse> {
        fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
            if let Some(tx) = tx {
                tx.send(self);
            }
        }
    }

pub struct LsDir {
    pub repo_key: String,
    pub reference: String,
    pub path: String,
}

impl Message for LsDir {
    type Result = LsDirResponse;
}

pub struct LsDirResponse(pub Result<Vec<String>, String>);

impl<A, M> MessageResponse<A, M> for LsDirResponse
    where A: Actor, M: Message<Result = LsDirResponse> {
        fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
            if let Some(tx) = tx {
                tx.send(self);
            }
        }
    }

pub struct GitRepos {
    repos: HashMap<String, Repository>,
    ops: Box<dyn GitOps>,
}

impl Actor for GitRepos {
    type Context = Context<Self>;
}

impl GitRepos {
    pub fn new(repos: HashMap<String, Repository>) -> GitRepos {
        GitRepos {
            repos,
            ops: Box::new(LibGitOps {}),
        }
    }
}

impl Handler<CatFile> for GitRepos {
    type Result = CatFileResponse;

    fn handle(&mut self, req: CatFile, _: &mut Self::Context) -> Self::Result {
        CatFileResponse(match self.repos.get(&req.repo_key) {
            Some(repo) => self
                .ops
                .cat_file(repo, &req.reference, &req.path)
                .map_err(|x| x.to_string()),
            None => Err(format!("No repo found with name '{}'", &req.repo_key)),
        })
    }
}

impl Handler<LsDir> for GitRepos {
    type Result = LsDirResponse;

    fn handle(&mut self, req: LsDir, _: &mut Self::Context) -> Self::Result {
        LsDirResponse(match self.repos.get(&req.repo_key) {
            Some(repo) => self
                .ops
                .ls_dir(repo, &req.reference, &req.path)
                .map_err(|x| x.to_string()),
            None => Err(format!("No repo found with name '{}'", &req.repo_key)),
        })
    }
}
