#[macro_use]
extern crate log;
extern crate env_logger;
extern crate clap;

extern crate rubbit;

use std::env;

use log::LogLevelFilter;
use env_logger::LogBuilder;

use rubbit::BuildStatus;
use rubbit::errors::*;
use rubbit::utils::RepoConfig;


fn run() -> Result<()> {
    init_logger()?;

    // Load config and program arguments
    let conf = rubbit::utils::load_config(None)?;
    let pargs = parse_args()?;

    let key = (pargs.owner.clone(), pargs.reponame.clone());
    let repo_conf: &RepoConfig = conf.repos.get(&key).ok_or(format!(
        "{}/{} not setup in configuration",
        pargs.owner.clone(),
        pargs.reponame.clone(),
    ))?;
    let integration = repo_conf.integration.clone();
    let api_token = repo_conf.api_token.clone();

    let build_request = rubbit::BuildRequest::new(
        integration,
        pargs.owner.clone(),
        pargs.reponame.clone(),
        pargs.commit.clone(),
    )?;

    info!("Received a new build-request: \"{:?}\"", build_request);
    let local_code = rubbit::LocalCode::new(&conf.meta.build_root, &build_request)
        .chain_err(|| {
            format!("Failed checking out code from {:?}", build_request)
        })?;

    let results = local_code.execute(
        &repo_conf.build_instruction,
    ).chain_err(|| "Failed execution of build")?;
    if !results.successful() {
        for (i, step) in results.steps.iter().enumerate() {
            println!("Step {} resulted in {:?}", i, step.status);
            println!("Output was:\n{}", step.output);
            if step.status != BuildStatus::Successful {}
        }
    }
    println!("Build-result: {:?}", results.successful());
    Ok(())
}

struct ProgramArgs {
    owner: String,
    reponame: String,
    commit: String,
}

fn parse_args() -> Result<ProgramArgs> {
    let matches = clap::App::new("rubbit-cli")
        .version("0.1")
        .about("CLI for rubbit build-server")
        .arg(
            clap::Arg::with_name("owner")
                .short("o")
                .long("owner")
                .value_name("OWNER")
                .help("Specify owner of repository")
                .required(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("repo")
                .short("r")
                .long("repo")
                .value_name("REPO")
                .help("Name of repository")
                .required(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("commit")
                .short("c")
                .long("commit")
                .value_name("COMMIT")
                .help("Which commit to build")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let get_arg = |arg: &str| {
        let res: Result<String> = Ok(
            matches
                .value_of_lossy(arg)
                .ok_or(format!("\"{}\" argument missing", arg))?
                .to_owned()
                .into_owned()
                .to_lowercase(),
        );
        res
    };
    let owner = get_arg("owner")?;
    let reponame = get_arg("repo")?;
    let commit = get_arg("commit")?;
    Ok(ProgramArgs {
        owner,
        reponame,
        commit,
    })
}

fn init_logger() -> Result<()> {

    let mut builder = LogBuilder::new();
    builder
        //.format(format)
        .filter(None, LogLevelFilter::Debug);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().chain_err(|| "Failed log-init")
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
