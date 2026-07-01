use crate::git;

const MAX_DIFF_CHARS: usize = 16_000;

pub struct Context {
    pub prompt: Option<String>,
    pub branch: String,
    pub change_source: Option<&'static str>,
    pub files: String,
    pub stat: String,
    pub diff: String,
    pub diff_truncated: bool,
}

impl Context {
    pub fn collect(prompt: Option<String>) -> anyhow::Result<Self> {
        let prompt = prompt
            .map(|prompt| prompt.trim().to_string())
            .filter(|prompt| !prompt.is_empty());

        let staged_files = git::run(&["diff", "--staged", "--name-status"])?
            .trim()
            .to_string();

        let (change_source, files, stat, diff) = if !staged_files.is_empty() {
            (
                Some("staged"),
                staged_files,
                git::run(&["diff", "--staged", "--stat"])?
                    .trim()
                    .to_string(),
                git::run(&["diff", "--staged", "--unified=3"])?
                    .trim()
                    .to_string(),
            )
        } else {
            let files = git::run(&["status", "--short", "--untracked-files=all"])?
                .trim_end()
                .to_string();

            if files.is_empty() {
                (None, String::new(), String::new(), String::new())
            } else {
                let stat = git::run(&["diff", "--stat"])?.trim().to_string();
                let diff = git::run(&["diff", "--unified=3"])?.trim().to_string();
                let stat = if diff.is_empty() {
                    String::from("Untracked-only changes; diff is empty until files are staged.")
                } else {
                    stat
                };

                (Some("unstaged"), files, stat, diff)
            }
        };

        if prompt.is_none() && files.is_empty() {
            anyhow::bail!("No staged or unstaged changes found.");
        }

        let (diff, diff_truncated) = truncate(diff, MAX_DIFF_CHARS);

        Ok(Self {
            prompt,
            branch: git::run(&["rev-parse", "--abbrev-ref", "HEAD"])?
                .trim()
                .to_string(),
            change_source,
            files,
            stat,
            diff,
            diff_truncated,
        })
    }
}

fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}
