use serde_json::{Value, json};

pub struct Settings {
    pub commit_convention: String,
    pub squash_on_merge: Decision,
    pub base_branch: String,
    pub open_as_draft: Decision,
    pub push_policy: String,
}

/// A yes/no policy answer that also allows a free-form response, e.g.
/// "squash into dev, then merge dev into main".
pub enum Decision {
    Yes,
    No,
    Custom(String),
}

impl Decision {
    pub fn from_answer(answer: String) -> Self {
        match answer.as_str() {
            "Yes" => Decision::Yes,
            "No" => Decision::No,
            _ => Decision::Custom(answer),
        }
    }

    /// Human-readable form used in the AI prompt.
    pub fn label(&self) -> &str {
        match self {
            Decision::Yes => "yes",
            Decision::No => "no",
            Decision::Custom(value) => value,
        }
    }

    /// JSON form: a bool for yes/no, a string for a free-form answer.
    fn as_json(&self) -> Value {
        match self {
            Decision::Yes => json!(true),
            Decision::No => json!(false),
            Decision::Custom(value) => json!(value),
        }
    }
}

impl Settings {
    pub fn to_json_string(&self) -> anyhow::Result<String> {
        let value = json!({
            "version": 1,
            "commit": {
                "convention": self.commit_convention.clone(),
            },
            "pull_request": {
                "squash_on_merge": self.squash_on_merge.as_json(),
                "open_as_draft": self.open_as_draft.as_json(),
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
    use super::{Decision, Settings};

    fn settings() -> Settings {
        Settings {
            commit_convention: "conventional".to_string(),
            squash_on_merge: Decision::Yes,
            base_branch: "main".to_string(),
            open_as_draft: Decision::No,
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

    #[test]
    fn serializes_custom_decision_as_string() {
        let mut settings = settings();
        settings.squash_on_merge = Decision::Custom("squash into dev, then merge dev".to_string());
        let value: serde_json::Value =
            serde_json::from_str(&settings.to_json_string().unwrap()).unwrap();

        assert_eq!(
            value["pull_request"]["squash_on_merge"],
            "squash into dev, then merge dev"
        );
    }

    #[test]
    fn from_answer_maps_yes_no_and_free_form() {
        assert!(matches!(
            Decision::from_answer("Yes".to_string()),
            Decision::Yes
        ));
        assert!(matches!(
            Decision::from_answer("No".to_string()),
            Decision::No
        ));
        assert!(matches!(
            Decision::from_answer("merge dev to main".to_string()),
            Decision::Custom(value) if value == "merge dev to main"
        ));
    }
}
