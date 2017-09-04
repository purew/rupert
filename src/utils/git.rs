use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use git2;
use git2::{Repository, Oid};

use errors::*;


pub fn fetch_origin_branches(repo: &Repository) -> Result<()> {
    info!("Fetching latest changes from origin.");
    Command::new("git")
        .current_dir(repo.path())
        .arg("fetch")
        .output()
        .chain_err(|| "Subprocess \"git fetch\" failed")?;
    Ok(())
    // TODO Switch to using libgit when implementing authentication
    //let mut remote = repo.find_remote("origin").chain_err(
    //    || "Failed remote-find",
    //)?;
    //let refspecs: Vec<String> = remote
    //    .refspecs()
    //    .filter_map(|v| v.str().map(|s| s.to_owned()))
    //    .collect();
    //let str_refs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();
    //remote.fetch(str_refs.as_slice(), None, None).chain_err(
    //    || "Failed fetching",
    //)
}

pub fn checkout(repo: &Repository, checksum: &str) -> Result<()> {
    // TODO Simplify function calls by using asref to handle both String and &str
    let path = repo.path().parent().unwrap();
    info!("Checking out {} ({:?})", checksum, &path);
    Command::new("git")
        .current_dir(path)
        .args(&["checkout", "--recurse-submodules", checksum])
        .output()
        .chain_err(|| "Subprocess \"git checkout\" failed")?;
    Ok(())

    // TODO Use git2 for this
    //let oid = Oid::from_str(checksum).chain_err(|| {
    //format!("Not a valid Oid: \"{}\"", checksum)
    //})?;
    //repo.reset(&oid, git2::ResetType::Hard, None).chain_err(|| "failed checkout")
}

pub fn init_repo(path: &Path, url: &str) -> Result<Repository> {
    match Repository::open(path) {
        Ok(repo) => {
            info!("Found local repo at {:?}.", path);
            Ok(repo)
        }
        Err(e) => {
            warn!(
                "Could not load local repository at {:?} due to {:?}, \
                attempting cloning",
                path,
                e.description()
            );
            Ok(clone_recurse(&url, &path).chain_err(
                || "Failed clone_recurse",
            )?)
        }
    }
}

fn clone_recurse(url: &str, path: &Path) -> Result<Repository> {
    info!("Cloning {} into {:?}.", url, path);
    // TODO Implement this with git2. Authentication is troublesome. Cargo' auth-code:
    // https://github.com/rust-lang/cargo/blob/def249f9c18280d84f29fd96978389689fb61051/src/cargo/sources/git/utils.rs#L360
    //Repository::clone_recurse(&url, &path)


    // For now, defer to system-git
    let output = Command::new("git")
        .arg("clone")
        .arg("--recursive")
        .arg("--jobs")
        .arg("8")
        .arg(url)
        .arg(path)
        .output()
        .chain_err(|| "Subprocess \"git clone\" failed")?;
    let repo = Repository::open(path).chain_err(
        || "Failed opening cloned repo",
    )?;
    Ok(repo)
}
