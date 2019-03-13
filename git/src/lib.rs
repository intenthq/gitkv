extern crate git2;

use git2::{Error, Repository};

pub fn cat_file(repo: &Repository, reference: &str, filename: &str) -> Result<String, Error> {
    let reference = repo.find_reference(reference)?;

    let tree = reference.peel_to_tree()?;

    let path = std::path::Path::new(filename);
    let te = tree.get_path(path)?;
    let blob = repo.find_blob(te.id())?;

    Ok(String::from_utf8_lossy(blob.content()).to_string())
}
