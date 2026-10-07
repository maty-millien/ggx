use anyhow::{Context, bail};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub(super) fn start_automatic_with(updater: &Path, marker: &Path, now: SystemTime, is_ci: bool) {
    if is_ci || !updater.is_file() || !is_due(marker_modified(marker), now) {
        return;
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
}

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
