use crate::git;

const MAX_DIFF_CHARS: usize = 16_000;

pub struct Context {
    pub branch: String,
    pub files: String,
    pub stat: String,
    pub diff: String,
    pub diff_truncated: bool,
}

impl Context {
    pub fn prompt() -> anyhow::Result<String> {
        Ok(Self::collect()?.render_prompt())
    }

    fn collect() -> anyhow::Result<Self> {
        let files = staged_files_or_stage_all()?;
        let diff = git::staged_diff()?;
        let (diff, diff_truncated) = truncate(diff, MAX_DIFF_CHARS);

        Ok(Self {
            branch: git::current_branch()?,
            files,
            stat: git::staged_diff_stat()?,
            diff,
            diff_truncated,
        })
    }

    fn render_prompt(&self) -> String {
        let truncation_note = if self.diff_truncated {
            "\n\n<Note>\nStaged diff was truncated.\n</Note>"
        } else {
            ""
        };

        format!(
            r#"<Instructions>
Generate a concise git commit message for the staged changes.
Use Conventional Commits when appropriate: feat, fix, refactor, docs, test, chore, build, ci.
Return only the commit message.
No markdown.
No explanation.
</Instructions>

<Data>

<Branch>
{}
</Branch>

<ChangedFiles>
{}
</ChangedFiles>

<DiffStat>
{}
</DiffStat>

<StagedDiff>
{}
</StagedDiff>{}

</Data>"#,
            self.branch, self.files, self.stat, self.diff, truncation_note
        )
    }
}

fn staged_files_or_stage_all() -> anyhow::Result<String> {
    let files = git::staged_files()?;

    if !files.is_empty() {
        return Ok(files);
    }

    git::stage_all()?;

    git::staged_files()
}

fn truncate(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    let truncated = value.chars().take(max_chars).collect();

    (truncated, true)
}
