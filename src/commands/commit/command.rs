use crate::commands::commit::context::Context;
use crate::git;

pub fn run() -> anyhow::Result<()> {
    if git::staged_files()?.is_empty() {
        git::stage_all()?;
    }

    println!("{}", Context::prompt()?);

    Ok(())
}
