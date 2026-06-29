use crate::git::run;

pub fn stage_all() -> anyhow::Result<()> {
    run(&["add", "--all"])?;

    Ok(())
}
