use actix::dev::{MessageResponse, ResponseChannel};
use actix::{Actor, Context, Handler, Message};
use git::{git2::Repository, GitOps, LibGitOps};
use std::collections::HashMap;

pub struct CatFile {
    pub repo_key: String,
    pub reference: String,
    pub filename: String,
}

impl CatFile {
    pub fn new(repo_key: String, reference: String, filename: String) -> CatFile {
        CatFile {
            repo_key,
            filename,
            reference,
        }
    }
}

impl Message for CatFile {
    type Result = CatFileResponse;
}

pub struct CatFileResponse(pub Result<Vec<u8>, String>);

impl<A, M> MessageResponse<A, M> for CatFileResponse
where
    A: Actor,
    M: Message<Result = CatFileResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

pub struct GitRepos {
    repos: HashMap<String, Repository>,
    ops: Box<GitOps>,
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

    fn handle(&mut self, task: CatFile, _: &mut Self::Context) -> Self::Result {
        CatFileResponse(match self.repos.get(&task.repo_key) {
            Some(repo) => self
                .ops
                .cat_file(repo, &task.reference, &task.filename)
                .map_err(|x| x.to_string()),
            None => Err("No repo found".to_string()),
        })
    }
}
