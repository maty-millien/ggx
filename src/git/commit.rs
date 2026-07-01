use crate::git::run;

pub fn commit(message: &str) -> anyhow::Result<()> {
    run(&["commit", "-m", message])?;

    Ok(())
}
