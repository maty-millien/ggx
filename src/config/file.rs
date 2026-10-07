use crate::ai::Provider;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const NOT_SETUP: &str = "ggx is not set up. Run `ggx setup` to choose an AI provider.";

pub(super) fn path_from(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    xdg.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .map(|base| base.join("ggx").join("config.json"))
}

pub(super) fn load_from(path: &Path) -> anyhow::Result<Provider> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => anyhow::bail!(NOT_SETUP),
        Err(error) => {
            anyhow::bail!(
                "Could not read ggx configuration at {}: {}. Run `ggx setup` again.",
                path.display(),
                error
            )
        }
    };

    parse(&contents).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid ggx configuration at {}. Run `ggx setup` again.",
            path.display()
        )
    })
}

fn parse(contents: &str) -> Option<Provider> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    Provider::parse(value.get("provider")?.as_str()?)
}
