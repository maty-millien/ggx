use crate::ai::Provider;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const NOT_SETUP: &str = "ggx is not set up. Run `ggx setup` to choose an AI provider.";

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

fn path() -> anyhow::Result<PathBuf> {
    path_from(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME")).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not locate the user configuration directory. Set XDG_CONFIG_HOME or HOME, then run `ggx setup`."
        )
    })
}

fn path_from(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    xdg.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .map(|base| base.join("ggx").join("config.json"))
}

fn load_from(path: &Path) -> anyhow::Result<Provider> {
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

#[cfg(test)]
mod tests {
    use super::{NOT_SETUP, load_from, parse, path_from};
    use crate::ai::Provider;
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn resolves_xdg_config_path() {
        assert_eq!(
            path_from(
                Some(OsString::from("/xdg")),
                Some(OsString::from("/home/user"))
            ),
            Some(Path::new("/xdg/ggx/config.json").to_path_buf())
        );
    }

    #[test]
    fn falls_back_to_home_config_path() {
        assert_eq!(
            path_from(None, Some(OsString::from("/home/user"))),
            Some(Path::new("/home/user/.config/ggx/config.json").to_path_buf())
        );
    }

    #[test]
    fn parses_known_providers() {
        assert_eq!(parse(r#"{"provider":"codex"}"#), Some(Provider::Codex));
        assert_eq!(parse(r#"{"provider":"claude"}"#), Some(Provider::Claude));
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert_eq!(parse("not json"), None);
        assert_eq!(parse(r#"{"provider":"other"}"#), None);
        assert_eq!(parse(r#"{"other":"claude"}"#), None);
    }

    #[test]
    fn reports_missing_configuration() {
        let error = load_from(Path::new("/path/that/does/not/exist/ggx.json")).unwrap_err();

        assert_eq!(error.to_string(), NOT_SETUP);
    }
}
