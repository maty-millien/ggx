use crate::{config, tui};
use anyhow::{Context, bail, ensure};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};
use std::{env, fs};

const INSTALLER_URL: &str =
    "https://github.com/maty-millien/ggx/releases/latest/download/ggx-installer.sh";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn start_automatic() {
    let (Ok(executable), Some(marker)) = (env::current_exe(), marker()) else {
        return;
    };
    let updater = executable.with_file_name("ggx-update");
    let due = fs::metadata(&marker)
        .and_then(|metadata| metadata.modified())
        .ok()
        .is_none_or(|checked| {
            SystemTime::now()
                .duration_since(checked)
                .is_ok_and(|elapsed| elapsed >= CHECK_INTERVAL)
        });
    if env::var_os("CI").is_some() || !updater.is_file() || !due {
        return;
    }

    let started = Command::new(&updater)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    if started {
        let _ = config::write(&marker, b"");
    }
}

pub fn run() -> anyhow::Result<()> {
    let executable = env::current_exe().context("Could not locate the ggx executable")?;
    let updater = executable.with_file_name("ggx-update");
    if !updater.is_file() {
        bail!(
            "ggx-update is missing; reinstall ggx with the official installer: curl --proto '=https' --tlsv1.2 -LsSf {INSTALLER_URL} | sh"
        );
    }

    let current_version = env!("CARGO_PKG_VERSION");
    tui::success("Current version", current_version);
    tui::rail();

    let (installed, elapsed) =
        tui::timed_spinner("Checking for updates", || install(&updater, &executable))?;
    if installed == current_version {
        tui::step("Update check complete", elapsed);
        tui::warning("Already up to date");
    } else {
        tui::success("New version", &installed);
        tui::rail();
        tui::success("Update complete", &format!("{:.1}s", elapsed.as_secs_f32()));
    }

    Ok(())
}

fn install(updater: &Path, executable: &Path) -> anyhow::Result<String> {
    let output = Command::new(updater)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("Could not start {}", updater.display()))?;
    if !output.status.success() {
        let [stderr, stdout] = [&output.stderr, &output.stdout]
            .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string());
        let detail = if stderr.is_empty() { stdout } else { stderr };
        if detail.is_empty() {
            bail!("ggx update failed with {}", output.status);
        }
        bail!("ggx update failed: {detail}");
    }

    let output = Command::new(executable)
        .arg("--version")
        .output()
        .context("Could not read the installed ggx version")?;
    ensure!(
        output.status.success(),
        "Could not read the installed ggx version"
    );
    let version = String::from_utf8(output.stdout)
        .ok()
        .and_then(|output| output.trim().strip_prefix("ggx ").map(str::to_owned))
        .context("ggx returned an invalid version")?;

    if let Some(marker) = marker() {
        let _ = config::write(&marker, b"");
    }
    Ok(version)
}

fn marker() -> Option<PathBuf> {
    config::dir("XDG_CACHE_HOME", ".cache").map(|cache| cache.join("ggx/update-check"))
}
