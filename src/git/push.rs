use crate::git::run;

pub fn push() -> anyhow::Result<()> {
    run(&["push"])?;

    Ok(())
}
