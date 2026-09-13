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

#[cfg(test)]
mod tests {
    use super::{NOT_SETUP, load_from, parse, path_from};
    use crate::ai::Provider;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};

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
    fn falls_back_to_home_config_path_when_xdg_is_unset_or_empty() {
        assert_eq!(
            path_from(None, Some(OsString::from("/home/user"))),
            Some(Path::new("/home/user/.config/ggx/config.json").to_path_buf())
        );
        assert_eq!(
            path_from(Some(OsString::new()), Some(OsString::from("/home/user"))),
            Some(Path::new("/home/user/.config/ggx/config.json").to_path_buf())
        );
    }

    #[test]
    fn has_no_path_without_environment_directories() {
        assert_eq!(path_from(None, None), None);
        assert_eq!(
            path_from(Some(OsString::new()), Some(OsString::new())),
            None
        );
    }

    #[test]
    fn parses_known_providers() {
        assert_eq!(parse(r#"{"provider":"codex"}"#), Some(Provider::Codex));
        assert_eq!(parse(r#"{"provider":"claude"}"#), Some(Provider::Claude));
        assert_eq!(parse(r#"{"provider":"copilot"}"#), Some(Provider::Copilot));
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

    #[test]
    fn reports_unreadable_configuration() {
        let error = load_from(Path::new("/")).unwrap_err().to_string();

        assert!(error.starts_with("Could not read ggx configuration at /:"));
        assert!(error.ends_with("Run `ggx setup` again."));
    }

    #[test]
    fn loads_provider_from_file_and_reports_invalid_contents() {
        let path = temp_path("config.json");

        fs::write(&path, "{\"provider\":\"claude\"}\n").unwrap();
        assert_eq!(load_from(&path).unwrap(), Provider::Claude);

        fs::write(&path, "{}").unwrap();
        let error = load_from(&path).unwrap_err().to_string();
        assert!(error.starts_with("Invalid ggx configuration at"));
        assert!(error.ends_with("Run `ggx setup` again."));

        let _ = fs::remove_file(path);
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ggx-config-test-{}-{name}", std::process::id()))
    }
}
