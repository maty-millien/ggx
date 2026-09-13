mod file;

use crate::ai::Provider;
use file::{load_from, path_from};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn load() -> anyhow::Result<Provider> {
    load_from(&path()?)
}

pub fn current() -> Option<Provider> {
    path().ok().and_then(|path| load_from(&path).ok())
}

pub fn save(provider: Provider) -> anyhow::Result<()> {
    let path = path()?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not locate the ggx configuration directory"))?;
    fs::create_dir_all(parent)?;
    fs::write(
        &path,
        format!("{{\"provider\":\"{}\"}}\n", provider.as_str()),
    )?;
    Ok(())
}

pub fn sibling(name: &str) -> anyhow::Result<PathBuf> {
    Ok(path()?.with_file_name(name))
}

fn path() -> anyhow::Result<PathBuf> {
    path_from(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME")).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not locate the user configuration directory. Set XDG_CONFIG_HOME or HOME, then run `ggx setup`."
        )
    })
}
