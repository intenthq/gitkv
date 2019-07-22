use actix::dev::MessageResponse;
use actix::{Actor, Context, Handler, Message};
use git::{git2::Repository, GitOps, LibGitOps};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Message)]
#[rtype(result="CatFileResponse")]
pub struct CatFile {
    pub repo_key: String,
    pub reference: String,
    pub path: PathBuf,
}

#[derive(MessageResponse)]
pub struct CatFileResponse(pub Result<Vec<u8>, String>);

#[derive(Message)]
#[rtype(result="LsDirResponse")]
pub struct LsDir {
    pub repo_key: String,
    pub reference: String,
    pub path: PathBuf,
}

#[derive(MessageResponse)]
pub struct LsDirResponse(pub Result<Vec<PathBuf>, String>);

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
