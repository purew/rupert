#[macro_use]
extern crate error_chain;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate git2;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

use std::error::Error;
use std::path::{Path, PathBuf};

use git2::Repository;

pub mod errors {
    error_chain!{
    errors {
        ParseError(v: String) {
            description("Parse-error")
            display("Parse-error: {}", v)
        }
    }}
}
use errors::*;

mod integrations;
pub mod utils;

use integrations::Integrations;


/// A request to build a certain commit
pub struct BuildRequest {
    integration: Integrations,
    owner: String,
    reponame: String,
    commit: String,
}

impl BuildRequest {
    pub fn new(
        integration: Integrations,
        owner: String,
        reponame: String,
        commit: String,
    ) -> Result<BuildRequest> {
        Ok(BuildRequest {
            owner,
            reponame,
            commit,
            integration,
        })
    }
}

/// repo:commit checked out on local path
pub struct LocalCode {
    root: PathBuf,
    repo: git2::Repository,
}

impl LocalCode {
    fn new(root: &Path, req: BuildRequest) -> Result<Self> {
        let url = integrations::get_integration_git_url(&req);
        let repo = utils::init_repo(&root, &url)?;
        utils::fetch_origin_branches(&repo)?;
        utils::checkout(&repo, &req.commit)?;
        let root = root.to_owned();
        Ok(LocalCode { root, repo })
    }
}

/// The `BuildConfig` containing all information needed for running tests
pub struct BuildConfig {
    request: BuildRequest,
    local_code: LocalCode,
    steps: Vec<BuildStep>,
}

/// Contains results of build
pub struct BuildResult {
    request: BuildRequest,
    steps: Vec<BuildStepResult>,
}

/// A single step in a build
pub struct BuildStep {
    cmd: String,
}

/// The result of executing a `BuildStep`
pub struct BuildStepResult {
    status: BuildStatus,
    cmd: String,
    output: String,
}

/// Status of a `BuildStepResult`
pub enum BuildStatus {
    Successful,
    Failed,
    InProgress,
    Stopped,
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
