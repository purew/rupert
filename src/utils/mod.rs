
use std::io::Read;
use std::fs::{DirBuilder, File, create_dir, read_dir, copy};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use toml;

use BuildInstruction;
use BuildStep;
use errors::*;
use integrations::Integrations;

pub mod git;

const FNAME_CONFIG: &'static str = "rubbit-conf.toml";


#[derive(Deserialize, Debug)]
struct RawConfig {
    meta: MetaConfig,
    repos: Vec<RepoConfig>,
}

#[derive(Deserialize, Debug)]
pub struct MetaConfig {
    pub build_root: PathBuf,
}

#[derive(Debug)]
pub struct Config {
    pub meta: MetaConfig,
    pub repos: HashMap<(String, String), RepoConfig>,
}

#[derive(Deserialize, Debug)]
pub struct RepoConfig {
    pub integration: Integrations,
    pub owner: String,
    pub reponame: String,
    pub api_token: String,
    pub build_instruction: BuildInstruction,
}


pub fn load_config(path: Option<PathBuf>) -> Result<Config> {
    let path = path.unwrap_or(Path::new(FNAME_CONFIG).into());
    let mut file = File::open(path).chain_err(|| {
        format!("Failed opening {}", FNAME_CONFIG)
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).chain_err(
        || "Could not read contents of file",
    )?;
    let mut raw: RawConfig = toml::from_str(&contents).chain_err(
        || "Bad format in config-file",
    )?;
    let meta = raw.meta;
    let mut repos = HashMap::new();
    for repo in raw.repos.into_iter() {
        let key = (repo.owner.clone(), repo.reponame.clone());
        repos.insert(key, repo);
    }
    Ok(Config { meta, repos })
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in read_dir(src).chain_err(
        || format!("Failed read_dir of {:?}", src),
    )?
    {
        let entry = entry.chain_err(|| "Failed reading entry")?;
        let path = entry.path();
        let mut subdst = dst.to_owned();
        subdst.push(path.file_name().ok_or(format!(
            "Could not get filename of {:?}",
            path
        ))?);
        if path.is_dir() {
            create_dir(&subdst).chain_err(|| {
                format!("Failed to create {:?}", subdst)
            })?;
            copy_dir(&path, &subdst).chain_err(
                || "Failed recursive copy",
            )?;
        } else {
            copy(&path, &subdst).chain_err(|| {
                format!("Failed copy of {:?} to {:?}", path, subdst)
            })?;
        }
    }
    Ok(())
}

pub fn prettify_command_output(raw: &[u8], indent_size: usize) -> String {
    String::from_utf8_lossy(raw).replace("\\n", "\n")
}

#[cfg(test)]
mod tests {

    use std::clone::Clone;
    use std::env::temp_dir;
    use std::fs::File;
    use std::fs::DirBuilder;
    use std::io::Write;
    use std::path::PathBuf;
    use std::env;

    use utils;

    lazy_static!{
        pub static ref TEST_DIR: PathBuf = {
            let mut path = env::temp_dir();
            path.push("rubbit-tests");
            DirBuilder::new().create(&path);
            path
        };
    }

    #[test]
    fn test_checkout() {
        // TODO Init new repo in /dev/shm, add two commits and verify `checkout` works
    }

    #[ignore]
    #[test]
    fn test_init_repo() {
        // TODO Make sure cloning a repo works
    }

    #[test]
    fn test_load_conf() {
        let mut path = TEST_DIR.clone();
        path.push("test_load_conf");
        //assert_eq!(path, PathBuf::new());
        let mut file = File::create(path.clone()).unwrap();
        file.write_all(
            b"
[meta]
build_root = \"/opt/rubbit/build_root\"

[[repos]]
integration = \"bitbucket\"
owner = \"purew\"
reponame = \"foobar\"
api_token = \"biggaboo\"
build_instruction = { steps = [
      {cmd = \"make\"},
      {cmd = \"make test\"},
    ]}
",
        ).unwrap();
        utils::load_config(Some(path)).unwrap();

    }
}
