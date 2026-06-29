use crate::commands::commit::context::Context;

pub fn run() -> anyhow::Result<()> {
    println!("{}", Context::prompt()?);

    Ok(())
}
