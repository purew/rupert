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
extern crate yansi;

#[macro_use]
extern crate log;

use std::error::Error;
use std::fs::{create_dir_all, remove_dir_all};
use std::hash::Hasher;
use std::io::{Bytes, Read};
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, Child, ChildStdout};

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
pub struct Runner {
    path_root: PathBuf,
    path_repo: PathBuf,
    path_build: PathBuf,
    path_cache: PathBuf,
    repo: git2::Repository,
    tx: Option<Sender<utils::BuildUpdates>>,
}

impl Runner {
    /// Initiate a new `Runner` object representing the local data on disk.
    ///
    /// Either clones repository or fetches and checks out commit in `BuildRequest`.
    pub fn new(
        rupert_root: &Path,
        req: &BuildRequest,
        tx: Option<Sender<utils::BuildUpdates>>,
    ) -> Result<Self> {
        let mut path_root = rupert_root.to_owned();
        path_root.push(&req.owner);
        path_root.push(&req.reponame);

        let path_repo = Runner::subdir(&path_root, "repo");
        let path_cache = Runner::subdir(&path_root, "cache");
        let path_build = Runner::path_build(&path_root, &req.commit);

        let url = req.build_clone_url();
        info!("init repo in {:?}", path_repo);
        let repo = utils::git::init_repo(&path_repo, &url)?;
        utils::git::fetch_origin_branches(&repo)?;
        utils::git::checkout(&repo, &req.commit)?;

        Ok(Runner {
            path_root,
            path_build,
            path_repo,
            path_cache,
            repo,
            tx,
        })
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
    pub fn execute(self, build_instruction: &BuildInstruction) -> Result<BuildResult> {
        self.prepare_dirs()?;
        info!("Executing build in {:?}", self.path_build);
        let mut results = Vec::new();
        for step in &build_instruction.steps {
            // TODO Clean env so Command runs without outside knowledge
            let step_result = self.spawn_step_worker(&step)?;
            let status = step_result.status.clone();
            results.push(step_result);
            if status != BuildStatus::Successful {
                break;
            }
        }
        Ok(BuildResult { steps: results })
    }

    fn spawn_step_worker(&self, step: &BuildStep) -> Result<BuildStepResult> {
        self.send_update(
            utils::BuildUpdates::StepStarted(step.cmd.clone()),
        )?;
        let mut child = Command::new("stdbuf")
            .current_dir(&self.path_build)
            .env("PATH_BUILD", &self.path_build)
            .env("PATH_CACHE", &self.path_cache)
            .args(&["-oL", "bash", "-c", &step.cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .chain_err(|| "Failed executing step")?;

        let mut status = BuildStatus::InProgress;
        let mut keep_waiting = true;
        let mut stdout_agg = String::new();
        let mut stderr_agg = String::new();
        while keep_waiting {
            match child.try_wait() {
                Ok(val) => {
                    let stdout = Runner::grab_line_from_stdout(&mut child)?;
                    if !stdout.is_empty() {
                        stdout_agg += &stdout;
                        self.send_update(utils::BuildUpdates::StepNewOutput(
                            utils::TextOutput::Stdout(stdout),
                        ))?;
                    }
                    if let Some(retval) = val {
                        let stderr = Runner::grab_stderr(&mut child)?;
                        if !stderr.is_empty() {
                            stderr_agg += &stderr;
                            self.send_update(utils::BuildUpdates::StepNewOutput(
                                utils::TextOutput::Stderr(format!(
                                    "------------- Rupert: Delayed stderr:\n{}\n------------- End of delayed stderr", stderr)
                                ),
                            ))?;
                        }
                        status = if retval.success() {
                            BuildStatus::Successful
                        } else {
                            BuildStatus::Failed
                        };
                        keep_waiting = false;
                    }
                }
                Err(e) => {
                    return Err(e).chain_err(|| "Could not wait on child-process");
                }
            };
        }
        Ok(BuildStepResult {
            status: status.clone(),
            cmd: step.cmd.clone(),
            output: stdout_agg,
        })
    }

    fn grab_line_from_stdout(child: &mut Child) -> Result<String> {
        let new_out = child
            .stdout
            .as_mut()
            .chain_err(|| "Could not grab stdout")?
            .bytes();
        let bytes = new_out
            .map(|v| v.chain_err(|| "Failed reading child output"))
            .take_while(|v| match v {
                &Ok(ref c) => {
                    //info!("stdout: {}", c);
                    if *c as usize == 10 { false } else { true }
                }
                _ => true,
            })
            .collect::<Result<Vec<u8>>>()?;
        let stdout = String::from_utf8_lossy(&bytes).trim().into();
        Ok(stdout)
    }
    fn grab_stderr(child: &mut Child) -> Result<String> {
        // TODO Currently grabbing all of stderr after child-process has finished
        // instead of interweaved with stdout as there is no "try_read" function.
        let mut buffer = Vec::new();
        child
            .stderr
            .as_mut()
            .chain_err(|| "Could not grab stderr")?
            .read_to_end(&mut buffer)
            .chain_err(|| "Failed reading child stderr")?;

        let stderr = String::from_utf8_lossy(&buffer).trim().into();
        Ok(stderr)
    }

    fn send_update(&self, update: utils::BuildUpdates) -> Result<()> {
        match &self.tx {
            &Some(ref tx) => {
                tx.send(update).chain_err(
                    || "Send of update to subscriber failed",
                )
            }
            &None => Ok(()),
        }
    }

    fn subdir(root: &PathBuf, name: &str) -> PathBuf {
        let mut path = root.to_owned();
        path.push(name);
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
}

/// The `BuildConfig` containing all information needed for running tests
pub struct BuildConfig {
    request: BuildRequest,
    runner: Runner,
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
#[derive(Clone, Deserialize, Debug)]
pub struct BuildInstruction {
    steps: Vec<BuildStep>,
}

/// A single step in a build
#[derive(Clone, Deserialize, Debug)]
pub struct BuildStep {
    cmd: String,
}

/// The result of executing a `BuildStep`
#[derive(Debug)]
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
