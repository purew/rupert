
use std::io::Read;
use std::fs::File;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use git2::{Repository, Oid};
use toml;

use errors::*;
use integrations::Integrations;


const FNAME_CONFIG: &'static str = "rustic-conf.toml";


#[derive(Deserialize, Debug)]
struct RawConfig {
    repos: Vec<RepoConfig>,
}

#[derive(Debug)]
pub struct Config {
    pub repos: HashMap<(String, String), RepoConfig>,
}

#[derive(Deserialize, Debug)]
pub struct RepoConfig {
    pub integration: Integrations,
    pub owner: String,
    pub reponame: String,
    pub api_token: String,
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
    let mut repos = HashMap::new();
    for repo in raw.repos.into_iter() {
        let key = (repo.owner.clone(), repo.reponame.clone());
        repos.insert(key, repo);
    }
    Ok(Config { repos })
}

pub fn fetch_origin_branches(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin").chain_err(
        || "Failed remote-find",
    )?;
    let refspecs: Vec<String> = remote
        .refspecs()
        .filter_map(|v| v.str().map(|s| s.to_owned()))
        .collect();
    let str_refs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();
    remote.fetch(str_refs.as_slice(), None, None).chain_err(
        || "Failed fetching",
    )
}

pub fn checkout(repo: &Repository, checksum: &str) -> Result<()> {
    // TODO Simplify function calls by using asref to handle both String and &str
    let oid = Oid::from_str(checksum).chain_err(|| {
        format!("Not a valid Oid: \"{}\"", checksum)
    })?;
    repo.set_head_detached(oid).chain_err(|| "failed checkout")
}

pub fn init_repo(path: &Path, url: &str) -> Result<Repository> {
    match Repository::init(path) {
        Ok(repo) => Ok(repo),
        Err(e) => {
            warn!(
                "Could not load local repository at {:?} due to {:?}, \
                attempting cloning",
                path,
                e.description()
            );
            Ok(Repository::clone_recurse(&url, &path).chain_err(
                || "Failed clone_recurse",
            )?)
        }
    }
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
            path.push("rustic-tests");
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
[config]
build_root = \"/opt/rustic/build_root\"

[[repos]]
integration = \"bitbucket\"
owner = \"purew\"
reponame = \"foobar\"
api_token = \"biggaboo\"",
        ).unwrap();
        utils::load_config(Some(path)).unwrap();

    }
}
