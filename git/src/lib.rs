pub extern crate git2;

use git2::{Error, Repository};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};

pub trait GitOps {
    fn cat_file(
        &self,
        repo: &Repository,
        reference: &str,
        path: &Path,
    ) -> Result<Vec<u8>, Error>;

    fn ls_dir(
        &self,
        repo: &Repository,
        reference: &str,
        path: &Path,
    ) -> Result<Vec<PathBuf>, Error>;
}

pub struct LibGitOps;

impl GitOps for LibGitOps {
    /// Given an existing git repository, it will read the blob that the reference and the filename
    /// point to and return it as a String.
    fn cat_file(
        &self,
        repo: &Repository,
        reference: &str,
        path: &Path,
    ) -> Result<Vec<u8>, Error> {
        let git_ref = repo.revparse_single(reference)?;
        let tree = git_ref.peel_to_tree()?;
        let te = tree.get_path(path)?;

        repo.find_blob(te.id()).map(|x| x.content().to_owned())
    }

    fn ls_dir(
        &self,
        repo: &Repository,
        reference: &str,
        directory: &Path,
    ) -> Result<Vec<PathBuf>, Error> {
        let git_ref = repo.revparse_single(reference)?;
        let tree = git_ref.peel_to_tree()?;
        let path = std::path::Path::new(directory);
        let te = tree.get_path(path)?;

        repo.find_tree(te.id()).map({ |tree|
            tree.iter().flat_map({ |tree_entry|
                tree_entry.name().map(|name| name.into())
            }).collect()
        })
    }
}

pub fn load_repos(root_path: &Path) -> HashMap<String, Repository> {
    fs::read_dir(root_path)
        .expect("Failed to read repos directory")
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_dir() {
                    let local_path = path.clone();
                    let repo_name = local_path
                        .file_stem()
                        .and_then(|name| name.to_os_string().into_string().ok());

                    repo_name.and_then(|name| {
                        Repository::open(path).ok().map(|repo| (name, repo))
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {

    extern crate tempdir;

    use super::{GitOps, LibGitOps};

    use git2::{Repository, Signature, Time};
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::str;

    // cat tests

    fn git_cat_file(
        repo_path: &Repository,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, git2::Error> {
        let gh = LibGitOps {};
        gh.cat_file(repo_path, reference, &PathBuf::from(path))
    }

    fn git_cat_file_err(repo_path: &Repository, reference: &str, path: &str) -> git2::Error {
        git_cat_file(repo_path, reference, path).expect_err("should be an error")
    }

    #[test]
    fn test_cat_file_with_valid_branch_ref_and_file() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res =
                git_cat_file(repo, "master", "dir/existing.file").expect("should be ok");
            assert_eq!(
                std::str::from_utf8(&res).expect("valid utf8"),
                "file content"
            );
        })
    }

    #[test]
    fn test_cat_file_with_valid_sha_ref_and_file() {
        with_repo("file content", "dir/existing.file", |repo, commit_sha| {
            let res =
                git_cat_file(repo, commit_sha, "dir/existing.file").expect("should be ok");
            assert_eq!(
                std::str::from_utf8(&res).expect("valid utf8"),
                "file content"
            );
        })
    }

    #[test]
    fn test_cat_file_with_valid_tag_ref_and_file() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res =
                git_cat_file(repo, "this-is-a-tag", "dir/existing.file").expect("should be ok");
            assert_eq!(
                std::str::from_utf8(&res).expect("valid utf8"),
                "file content"
            );
        })
    }

    #[test]
    fn test_cat_file_with_non_existing_ref() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_cat_file_err(repo, "idonot/exist", "dir/existing.file");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Reference);
        })
    }

    #[test]
    fn test_cat_file_with_non_existing_file() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_cat_file_err(repo, "master", "non-existing.file");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Tree);
        })
    }

    #[test]
    fn test_cat_file_with_dir() {
        with_repo("content", "dir/existing.file", |repo, _| {
            let res = git_cat_file_err(repo, "master", "dir");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Invalid);
        })
    }

    // ls tests

    // Converts a vec of string like things into a vec of owned paths.
    macro_rules! as_path_bufs {
        ($vec: expr) => {
            {
                $vec.iter().map(PathBuf::from).collect::<Vec<_>>()
            }
        };
    }

    fn git_ls_dir(
        repo_path: &Repository,
        reference: &str,
        path: &str,
    ) -> Result<Vec<PathBuf>, git2::Error> {
        let gh = LibGitOps {};
        gh.ls_dir(repo_path, reference, &PathBuf::from(path))
    }

    fn git_ls_dir_err(repo_path: &Repository, reference: &str, directory: &str) -> git2::Error {
        git_ls_dir(repo_path, reference, directory).expect_err("should be an error")
    }

    #[test]
    fn test_ls_dir_with_valid_branch_ref_and_dir() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_ls_dir(repo, "master", "dir").expect("should be ok");
            assert_eq!(res, as_path_bufs!(vec!["existing.file"]));
        })
    }

    #[test]
    fn test_ls_dir_with_valid_sha_ref_and_file() {
        with_repo("file content", "dir/existing.file", |repo, commit_sha| {
            let res = git_ls_dir(repo, commit_sha, "dir").expect("should be ok");
            assert_eq!(res, as_path_bufs!(vec!["existing.file"]));
        })
    }

    #[test]
    fn test_ls_dir_with_valid_tag_ref_and_file() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_ls_dir(repo, "this-is-a-tag", "dir").expect("should be ok");
            assert_eq!(res, as_path_bufs!(vec!["existing.file"]));
        })
    }

    #[test]
    fn test_ls_dir_with_non_existing_ref() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_ls_dir_err(repo, "idonot/exist", "dir");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Reference);
        })
    }

    #[test]
    fn test_ls_dir_with_non_existing_dir() {
        with_repo("file content", "dir/existing.file", |repo, _| {
            let res = git_ls_dir_err(repo, "master", "non-existing");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Tree);
        })
    }

    #[test]
    fn test_ls_dir_with_file() {
        with_repo("content", "dir/existing.file", |repo, _| {
            let res = git_ls_dir_err(repo, "master", "dir/existing.file");
            assert_eq!(res.code(), git2::ErrorCode::NotFound);
            assert_eq!(res.class(), git2::ErrorClass::Invalid);
        })
    }

    pub fn with_repo<F>(file_contents: &str, file: &str, callback: F)
    where
        F: Fn(&Repository, &str),
    {
        let dir = tempdir::TempDir::new("testgitrepo").expect("can't create tmp dir");

        let repo = Repository::init(&dir).expect("can't initialise repository");

        let path = dir.path().join(file);
        path.parent().map(|parent| fs::create_dir_all(&parent));
        fs::File::create(path)
            .and_then(|mut file| file.write_all(file_contents.as_bytes()))
            .expect("can't write file contents");

        let time = Time::new(123_456_789, 0);
        let sig = Signature::new("Foo McBarson", "foo.mcbarson@iamarealboy.net", &time)
            .expect("couldn't create signature for commit");

        let commit_oid = repo.index()
            .and_then(|mut index| {
                index
                    .add_path(Path::new(file))
                    .expect("can't add file to index");

                index
                    .write_tree()
                    .and_then(|tid| repo.find_tree(tid))
                    .and_then(|tree| {
                        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                    })
            }).expect("can't do first commit");

        let commit = repo.find_object(commit_oid, None)
            .expect("Could not find first commit.");
        repo.tag("this-is-a-tag", &commit, &sig, "This is a tag.", false)
            .expect("Could not create tag.");

        let commit_sha = format!("{}", commit_oid);

        callback(&repo, &commit_sha);
        dir.close().expect("couldn't close the dir");
    }

}
