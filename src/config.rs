use crate::ai::Provider;
use anyhow::Context;
use clap::ValueEnum;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::{env, fs};

const NOT_SETUP: &str = "ggx is not set up. Run `ggx setup` to choose an AI provider.";

pub fn load() -> anyhow::Result<Provider> {
    let path = path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => anyhow::bail!(NOT_SETUP),
        Err(error) => anyhow::bail!(
            "Could not read ggx configuration at {}: {error}. Run `ggx setup` again.",
            path.display()
        ),
    };

    serde_json::from_str::<serde_json::Value>(&contents)
        .ok()
        .and_then(|value| Provider::from_str(value["provider"].as_str()?, false).ok())
        .with_context(|| {
            format!(
                "Invalid ggx configuration at {}. Run `ggx setup` again.",
                path.display()
            )
        })
}

pub fn save(provider: Provider) -> anyhow::Result<()> {
    let contents = format!("{{\"provider\":\"{}\"}}\n", provider.name());
    Ok(write(&path()?, contents)?)
}

/// A file next to the configuration file.
pub fn sibling(name: &str) -> anyhow::Result<PathBuf> {
    Ok(path()?.with_file_name(name))
}

/// `$var` when set, otherwise `$HOME/<home_subdir>`.
pub fn dir(var: &str, home_subdir: &str) -> Option<PathBuf> {
    let non_empty = |name: &str| {
        env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    non_empty(var).or_else(|| non_empty("HOME").map(|home| home.join(home_subdir)))
}

/// Writes a file, creating its parent directories.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn path() -> anyhow::Result<PathBuf> {
    dir("XDG_CONFIG_HOME", ".config")
        .map(|base| base.join("ggx/config.json"))
        .context("Could not locate the user configuration directory. Set XDG_CONFIG_HOME or HOME, then run `ggx setup`.")
}
