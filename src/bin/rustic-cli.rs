extern crate rustic;

use rustic::errors::*;
use rustic::utils::RepoConfig;

fn run() -> Result<()> {
    let conf = rustic::utils::load_config(None)?;

    // TODO Parse owner/repo/commit from commandline
    let owner = "purew".to_owned();
    let reponame = "evroutes-v3".to_owned();
    let commit = "353314e5ea4953be54f96ff4bbe820933d34354c".to_owned();

    let key = (owner.clone(), reponame.clone());
    let repo_conf: &RepoConfig = conf.repos.get(&key).ok_or(format!(
        "{}/{} not setup in configuration",
        owner,
        reponame
    ))?;
    let integration = repo_conf.integration.clone();
    let api_token = repo_conf.api_token.clone();

    let build_request = rustic::BuildRequest::new(integration, owner, reponame, commit)?;

    println!("conf: {:?}", conf);
    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}
