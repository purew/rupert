

use serde_json::Value;


use errors::*;
use BuildRequest;
mod bitbucket;

#[derive(Deserialize, Debug, Clone)]
pub enum Integrations {
    #[serde(rename = "bitbucket")]
    Bitbucket,
}


pub fn get_integration_git_url(repo_update: &BuildRequest) -> String {
    match repo_update.integration {
        Integrations::Bitbucket => {
            format!(
                "https://bitbucket.org/{}/{}",
                repo_update.owner,
                repo_update.reponame
            )
        }
    }
}

pub trait Hookable {
    fn parse_push_request(val: Value) -> Result<BuildRequest>;
}
