use anyhow::{Context, bail};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub(super) fn start_automatic_with(
    updater: &Path,
    marker: &Path,
    now: SystemTime,
    is_ci: bool,
) -> bool {
    if is_ci || !updater.is_file() || !is_due(marker_modified(marker), now) {
        return false;
    }

    let started = Command::new(updater)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();

    if started {
        let _ = touch(marker);
    }

    started
}

#[derive(Debug, PartialEq)]
pub(super) enum UpdateOutcome {
    Current,
    Updated(String),
}

pub(super) fn perform_update(
    updater: &Path,
    executable: &Path,
    marker: Option<&Path>,
    current_version: &str,
) -> anyhow::Result<UpdateOutcome> {
    let output = Command::new(updater)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("Could not start {}", updater.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        if detail.is_empty() {
            bail!("ggx update failed with {}", output.status);
        }
        bail!("ggx update failed: {detail}");
    }

    let installed_version = installed_version(executable)?;
    if let Some(marker) = marker {
        let _ = touch(marker);
    }

    if installed_version == current_version {
        Ok(UpdateOutcome::Current)
    } else {
        Ok(UpdateOutcome::Updated(installed_version))
    }
}

fn installed_version(executable: &Path) -> anyhow::Result<String> {
    let output = Command::new(executable)
        .arg("--version")
        .output()
        .context("Could not read the installed ggx version")?;
    if !output.status.success() {
        bail!("Could not read the installed ggx version");
    }

    let output = String::from_utf8(output.stdout).context("ggx returned an invalid version")?;
    output
        .trim()
        .strip_prefix("ggx ")
        .map(str::to_owned)
        .context("ggx returned an invalid version")
}

pub(super) fn cache_marker_path(
    xdg_cache: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    let cache = xdg_cache
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|path| !path.is_empty())
                .map(|path| PathBuf::from(path).join(".cache"))
        })?;

    Some(cache.join("ggx/update-check"))
}

fn marker_modified(marker: &Path) -> Option<SystemTime> {
    fs::metadata(marker)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn is_due(modified: Option<SystemTime>, now: SystemTime) -> bool {
    modified.is_none_or(|modified| {
        now.duration_since(modified)
            .is_ok_and(|elapsed| elapsed >= CHECK_INTERVAL)
    })
}

fn touch(marker: &Path) -> std::io::Result<()> {
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker, [])
}

#[cfg(test)]
mod tests {
    use super::{
        CHECK_INTERVAL, UpdateOutcome, cache_marker_path, is_due, perform_update,
        start_automatic_with,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn update_is_due_without_marker() {
        assert!(is_due(None, UNIX_EPOCH + CHECK_INTERVAL));
    }

    #[test]
    fn update_is_not_due_before_interval() {
        let now = UNIX_EPOCH + CHECK_INTERVAL;

        assert!(!is_due(Some(now - Duration::from_secs(1)), now));
    }

    #[test]
    fn update_is_due_at_interval() {
        let now = UNIX_EPOCH + CHECK_INTERVAL;

        assert!(is_due(Some(UNIX_EPOCH), now));
    }

    #[test]
    fn update_is_not_due_when_marker_is_in_future() {
        let now = UNIX_EPOCH + CHECK_INTERVAL;

        assert!(!is_due(Some(now + Duration::from_secs(1)), now));
    }

    #[test]
    fn cache_prefers_xdg_directory() {
        assert_eq!(
            cache_marker_path(Some(OsString::from("/xdg")), Some(OsString::from("/home"))),
            Some(PathBuf::from("/xdg/ggx/update-check"))
        );
    }

    #[test]
    fn cache_falls_back_to_home() {
        assert_eq!(
            cache_marker_path(None, Some(OsString::from("/home/user"))),
            Some(PathBuf::from("/home/user/.cache/ggx/update-check"))
        );
    }

    #[test]
    fn cache_is_unavailable_without_environment_paths() {
        assert_eq!(cache_marker_path(None, None), None);
    }

    #[test]
    fn automatic_update_skips_missing_updater() {
        let directory = test_directory("missing");

        assert!(!start_automatic_with(
            &directory.join("missing-updater"),
            &directory.join("marker"),
            SystemTime::now(),
            false,
        ));
    }

    #[test]
    fn automatic_update_skips_ci() {
        let directory = test_directory("ci");
        let updater = script(&directory, "updater", "exit 0");
        let marker = directory.join("marker");

        assert!(!start_automatic_with(
            &updater,
            &marker,
            SystemTime::now(),
            true,
        ));
        assert!(!marker.exists());
    }

    #[test]
    fn automatic_update_starts_updater_and_marks_check() {
        let directory = test_directory("automatic");
        let updater = script(&directory, "updater", "exit 0");
        let marker = directory.join("marker");

        assert!(start_automatic_with(
            &updater,
            &marker,
            SystemTime::now(),
            false,
        ));
        assert!(marker.is_file());
        assert!(!start_automatic_with(
            &updater,
            &marker,
            SystemTime::now(),
            false,
        ));
    }

    #[test]
    fn manual_update_reports_new_version() {
        let directory = test_directory("updated");
        let updater = script(&directory, "updater", "exit 0");
        let executable = script(&directory, "ggx", "echo 'ggx 2.0.0'");
        let marker = directory.join("marker");

        assert_eq!(
            perform_update(&updater, &executable, Some(&marker), "1.0.0").unwrap(),
            UpdateOutcome::Updated("2.0.0".to_string())
        );
        assert!(marker.is_file());
    }

    #[test]
    fn manual_update_reports_current_version() {
        let directory = test_directory("current");
        let updater = script(&directory, "updater", "exit 0");
        let executable = script(&directory, "ggx", "echo 'ggx 1.0.0'");

        assert_eq!(
            perform_update(&updater, &executable, None, "1.0.0").unwrap(),
            UpdateOutcome::Current
        );
    }

    #[test]
    fn manual_update_propagates_updater_error() {
        let directory = test_directory("failed");
        let updater = script(
            &directory,
            "updater",
            "echo 'network unavailable' >&2; exit 7",
        );
        let executable = script(&directory, "ggx", "echo 'ggx 1.0.0'");

        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();

        assert!(error.to_string().contains("network unavailable"));
    }

    #[test]
    fn manual_update_reports_missing_updater() {
        let directory = test_directory("no-updater");
        let updater = directory.join("missing");
        let executable = script(&directory, "ggx", "echo 'ggx 1.0.0'");

        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("Could not start {}", updater.display())
        );
    }

    #[test]
    fn manual_update_reports_stdout_or_status_when_stderr_is_empty() {
        let directory = test_directory("failed-quiet");
        let executable = script(&directory, "ggx", "echo 'ggx 1.0.0'");

        let updater = script(&directory, "updater-stdout", "echo 'rate limited'; exit 1");
        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();
        assert_eq!(error.to_string(), "ggx update failed: rate limited");

        let updater = script(&directory, "updater-silent", "exit 3");
        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();
        assert_eq!(error.to_string(), "ggx update failed with exit status: 3");
    }

    #[test]
    fn manual_update_rejects_unreadable_installed_version() {
        let directory = test_directory("bad-version");
        let updater = script(&directory, "updater", "exit 0");

        let executable = script(&directory, "ggx-failing", "exit 1");
        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Could not read the installed ggx version"
        );

        let executable = script(&directory, "ggx-garbage", "echo 'something else'");
        let error = perform_update(&updater, &executable, None, "1.0.0").unwrap_err();
        assert_eq!(error.to_string(), "ggx returned an invalid version");
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/update-tests")
            .join(format!("{}-{name}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn script(directory: &Path, name: &str, body: &str) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}
