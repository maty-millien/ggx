use crate::commands::commit::context::Context;
use crate::git;

pub fn run() -> anyhow::Result<()> {
    ensure_staged_changes()?;

    println!("{}", Context::prompt()?);

    Ok(())
}

fn ensure_staged_changes() -> anyhow::Result<()> {
    if git::staged_files()?.is_empty() {
        git::stage_all()?;
    }

    Ok(())
}
