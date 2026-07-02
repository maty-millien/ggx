use crate::vcs::{git, github};

const MAX_DIFF_CHARS: usize = 16_000;
const MAX_ISSUE_BODY_CHARS: usize = 8_000;

pub struct Context {
    pub branch: String,
    pub base: String,
    pub files: String,
    pub stat: String,
    pub numstat: String,
    pub commits: String,
    pub diff: String,
    pub issues: Vec<Issue>,
}

pub struct Issue {
    pub reference: String,
    pub number: String,
    pub title: String,
    pub body: String,
    pub url: String,
}

impl Context {
    pub fn collect(base: Option<String>, closes: Vec<String>) -> anyhow::Result<Self> {
        let branch = git::current_branch()?;
        let base = base
            .map(|base| base.trim().to_string())
            .filter(|base| !base.is_empty())
            .map(Ok)
            .unwrap_or_else(git::default_base)?;

        if branch == base {
            anyhow::bail!("Current branch is already the base branch '{}'.", base);
        }

        let base_ref = git::base_ref(&base)?;
        let files = git::run(&["diff", "--name-status", &format!("{}...HEAD", base_ref)])?
            .trim()
            .to_string();
        let stat = git::run(&["diff", "--stat", &format!("{}...HEAD", base_ref)])?
            .trim()
            .to_string();
        let numstat = git::run(&["diff", "--numstat", &format!("{}...HEAD", base_ref)])?
            .trim()
            .to_string();
        let commits = git::run(&["log", "--oneline", &format!("{}..HEAD", base_ref)])?
            .trim()
            .to_string();
        let diff = git::run(&["diff", "--unified=3", &format!("{}...HEAD", base_ref)])?
            .trim()
            .to_string();

        if files.is_empty() && commits.is_empty() {
            anyhow::bail!("No changes found between {} and {}.", base, branch);
        }

        let (diff, _) = truncate(diff, MAX_DIFF_CHARS);
        let issues = closes
            .into_iter()
            .map(collect_issue)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self {
            branch,
            base,
            files,
            stat,
            numstat,
            commits,
            diff,
            issues,
        })
    }
}

fn collect_issue(reference: String) -> anyhow::Result<Issue> {
    let issue = github::issue(&reference)?;
    let (body, _) = truncate(issue.body, MAX_ISSUE_BODY_CHARS);

    Ok(Issue {
        reference,
        number: issue.number,
        title: issue.title,
        body,
        url: issue.url,
    })
}

fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_keeps_short_value() {
        let (value, truncated) = truncate("short".to_string(), 10);

        assert_eq!(value, "short");
        assert!(!truncated);
    }

    #[test]
    fn truncate_tracks_char_boundary() {
        let (value, truncated) = truncate("éclair".to_string(), 2);

        assert_eq!(value, "éc");
        assert!(truncated);
    }
}
