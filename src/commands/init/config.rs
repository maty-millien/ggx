use serde_json::json;

pub struct Settings {
    pub commit_convention: String,
    pub squash_on_merge: bool,
    pub base_branch: String,
    pub open_as_draft: bool,
    pub push_policy: String,
}

impl Settings {
    pub fn to_json_string(&self) -> anyhow::Result<String> {
        let value = json!({
            "version": 1,
            "commit": {
                "convention": self.commit_convention.clone(),
            },
            "pull_request": {
                "squash_on_merge": self.squash_on_merge,
                "open_as_draft": self.open_as_draft,
                "base_branch": self.base_branch.clone(),
            },
            "push": {
                "policy": self.push_policy.clone(),
            },
        });

        Ok(serde_json::to_string_pretty(&value)?)
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    fn settings() -> Settings {
        Settings {
            commit_convention: "conventional".to_string(),
            squash_on_merge: true,
            base_branch: "main".to_string(),
            open_as_draft: false,
            push_policy: "pr-only".to_string(),
        }
    }

    #[test]
    fn serializes_expected_shape() {
        let json = settings().to_json_string().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["version"], 1);
        assert_eq!(value["commit"]["convention"], "conventional");
        assert_eq!(value["pull_request"]["squash_on_merge"], true);
        assert_eq!(value["pull_request"]["open_as_draft"], false);
        assert_eq!(value["pull_request"]["base_branch"], "main");
        assert_eq!(value["push"]["policy"], "pr-only");
    }
}
