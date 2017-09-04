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
use std::fs::{create_dir_all, remove_dir_all};
use std::hash::Hasher;
use std::thread::sleep;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use git2::Repository;

use integrations::Hookable;

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
#[derive(Serialize, Deserialize, Debug)]
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
    path_root: PathBuf,
    path_repo: PathBuf,
    path_build: PathBuf,
    path_cache: PathBuf,
    repo: git2::Repository,
}

impl LocalCode {

    /// Initiate a new `LocalCode` object representing the local data on disk.
    ///
    /// Either clones repository or fetches and checks out commit in `BuildRequest`.
    pub fn new(rubbit_root: &Path, req: &BuildRequest) -> Result<Self> {
        let mut path_root = rubbit_root.to_owned();
        path_root.push(&req.owner);
        path_root.push(&req.reponame);

        let path_repo = LocalCode::subdir(&path_root, "repo");
        let path_cache = LocalCode::subdir(&path_root, "cache");
        let path_build = LocalCode::path_build(&path_root, &req.commit);

        let url = req.build_clone_url();
        let repo = utils::git::init_repo(&path_repo, &url)?;
        utils::git::fetch_origin_branches(&repo)?;
        utils::git::checkout(&repo, &req.commit)?;

        Ok(LocalCode { path_root, path_build, path_repo, path_cache, repo})
    }

    fn subdir(root: &PathBuf, name: &str) -> PathBuf {
        let mut path = root.to_owned();
        path.push("cache");
        path
    }

    fn path_build(root: &PathBuf, commit: &str) -> PathBuf {
        let mut path = root.clone();
        path.push("builds");
        // TODO Do we want each build in a new dir?
        //path.push(commit);
        path.push("common");
        path
    }

    fn prepare_dirs(&self) -> Result<()> {
        if self.path_build.exists() {
            let res = remove_dir_all(&self.path_build).chain_err(|| {
                format!("Failed remove of {:?}", self.path_build)
            });
            if let Err(e) = res {
                warn!(
                    "Could not remove old build in {:?} due to {:?}",
                    self.path_build,
                    e
                );
            }
        }
        create_dir_all(&self.path_build).chain_err(|| {
            format!("Failed creating build-dir: {:?}", self.path_build)
        })?;

        utils::copy_dir(&self.path_repo, &self.path_build)?;
        create_dir_all(&self.path_cache).chain_err(|| {
            format!("Failed creating cache-dir: {:?}", self.path_cache)
        })?;
        Ok(())
    }

    /// Execute build-steps from configuration on local code
    pub fn execute(
        self,
        build_instruction: &BuildInstruction,
    ) -> Result<BuildResult> {

        self.prepare_dirs()?;

        info!("Executing build in {:?}", self.path_build);
        let mut results = Vec::new();
        for step in &build_instruction.steps {
            // TODO Clean env so Command runs without outside knowledge
            info!("Executing child with {}", &step.cmd);
            let mut child = Command::new("bash")
                .current_dir(&self.path_build)
                .env("BUILD_PATH", &self.path_build)
                .args(&["-c", &step.cmd])
                .spawn()
                .chain_err(|| "Failed executing step")?;

            let mut status = BuildStatus::InProgress;
            let mut keep_waiting = true;
            while keep_waiting {
                match child.try_wait() {
                    Ok(Some(retval)) => {
                        status = if retval.success() {
                            BuildStatus::Successful
                        } else {
                            BuildStatus::Failed
                        };
                        keep_waiting = false;
                    }
                    Ok(None) => {
                        // TODO Add timeout
                        sleep(Duration::new(1, 0));
                    }
                    Err(e) => {
                        return Err(e).chain_err(|| "Could not wait on child-process");
                    }
                };
            }
            let indent = "    ";
            let out = "".to_owned();
            //format!("===== STDOUT =====\n{}{:?}\n===== STDERR =====\n{}{:?}\n=====",
            //                  &indent,
            //                  "",
            //                     //utils::prettify_command_output(&output.stdout, indent.len()),
            //                  &indent,
            //                  "",
            //                     //utils::prettify_command_output(&output.stderr, indent.len()));
            let step_result = BuildStepResult {
                status: status.clone(),
                cmd: step.cmd.clone(),
                output: out,
            };
            results.push(step_result);
            if status != BuildStatus::Successful {
                break;
            }
        }
        Ok(BuildResult { steps: results })
    }
}

/// The `BuildConfig` containing all information needed for running tests
pub struct BuildConfig {
    request: BuildRequest,
    local_code: LocalCode,
    build_instruction: BuildInstruction,
}

/// Contains results of build
pub struct BuildResult {
    pub steps: Vec<BuildStepResult>,
}

impl BuildResult {
    pub fn successful(&self) -> bool {
        !self.steps.iter().any(
            |res| res.status != BuildStatus::Successful,
        )
    }
}

/// A single step in a build
#[derive(Deserialize, Debug)]
pub struct BuildInstruction {
    steps: Vec<BuildStep>,
}

/// A single step in a build
#[derive(Deserialize, Debug)]
pub struct BuildStep {
    cmd: String,
}

/// The result of executing a `BuildStep`
pub struct BuildStepResult {
    pub status: BuildStatus,
    pub cmd: String,
    pub output: String,
}

/// Status of a `BuildStepResult`
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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
