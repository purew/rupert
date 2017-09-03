/// Integration for Bitbucket
///
/// API for updating build-status:
/// https://developer.atlassian.com/bitbucket/api/2/reference/resource/repositories/%7Busername%7D/%7Brepo_slug%7D/commit/%7Bnode%7D/statuses/build
///
/// Documentation on web-hooks:
/// https://confluence.atlassian.com/bitbucket/manage-webhooks-735643732.html
///

use serde_json::{value, Value};

use errors::*;
use BuildRequest;
use integrations::{Hookable, Integrations};


impl Hookable for BuildRequest {
    fn parse_push_request(val: Value) -> Result<BuildRequest> {
        let integration = Integrations::Bitbucket;
        let owner = json_val_as_str(&val, "actor")?;
        let reponame = json_val_as_str(&val, "repository")?;
        let pushval = json_val_as_val(&val, "push")?;
        let changesval = json_val_as_val(&pushval, "changes")?;
        let firstrefval = json_arr_as_val(&changesval, 0)?;
        let newval = json_val_as_val(&firstrefval, "new")?;
        let targetval = json_val_as_val(&newval, "target")?;
        let commit = json_val_as_str(&targetval, "hash")?;

        Ok(BuildRequest {
            reponame,
            commit,
            integration,
            owner,
        })
    }
    fn build_clone_url(&self) -> String {
        format!("git@bitbucket.org:{}/{}.git", self.owner, self.reponame)
    }
}

fn json_val_as_str(val: &Value, key: &str) -> Result<String> {
    Ok(
        json_val_as_val(&val, key)?
            .as_str()
            .ok_or(ErrorKind::ParseError(
                format!("\"{}\" is not string", key).into(),
            ))?
            .to_owned(),
    )
}

fn json_val_as_val<'a>(val: &'a Value, key: &str) -> Result<&'a Value> {
    val.get(key).ok_or(
        ErrorKind::ParseError(format!("No \"{}\" in json", key).into()).into(),
    )
}

fn json_arr_as_val<'a>(val: &'a Value, index: usize) -> Result<&'a Value> {
    val.get(index).ok_or(
        ErrorKind::ParseError(format!("No \"{}\" in json", index).into()).into(),
    )
}


#[cfg(test)]
mod tests {

    use serde_json;
    use serde_json::Value;

    use BuildRequest;
    use integrations::Hookable;

    lazy_static!{
        static ref PUSH_EXAMPLE: Value = serde_json::from_str(include_str!("bitbucket-push-example.json")).unwrap();
    }

    #[test]
    fn parse_example_build_request() {
        BuildRequest::parse_push_request(PUSH_EXAMPLE.clone()).unwrap();
    }
}
