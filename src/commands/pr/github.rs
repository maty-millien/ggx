use std::process::{Command, Stdio};

pub struct Issue {
    pub number: String,
    pub title: String,
    pub body: String,
    pub url: String,
}

pub fn issue(reference: &str) -> anyhow::Result<Issue> {
    let output = gh(&[
        "issue",
        "view",
        reference,
        "--json",
        "number,title,body,url",
    ])?;
    let value: serde_json::Value = serde_json::from_str(&output)?;

    Ok(Issue {
        number: json_string(&value, "number"),
        title: json_string(&value, "title"),
        body: json_string(&value, "body"),
        url: json_string(&value, "url"),
    })
}

pub fn create_pr(
    base: &str,
    branch: &str,
    title: &str,
    body: &str,
    draft: bool,
) -> anyhow::Result<String> {
    let mut args = vec![
        "pr", "create", "--base", base, "--head", branch, "--title", title, "--body", body,
    ];

    if draft {
        args.push("--draft");
    }

    Ok(gh(&args)?.trim().to_string())
}

fn gh(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| anyhow::anyhow!("failed to run gh: {}", error))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    anyhow::bail!("gh {} failed: {}", args.join(" "), stderr);
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .map(|value| match value {
            serde_json::Value::String(value) => value.clone(),
            serde_json::Value::Number(value) => value.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}
