

use serde_json::Value;


use errors::*;
use BuildRequest;
mod bitbucket;

pub trait Hookable {
    fn parse_push_request(val: Value) -> Result<BuildRequest>;
    fn build_clone_url(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Integrations {
    #[serde(rename = "bitbucket")]
    Bitbucket,
}
