mod check;

use crate::tui;
use anyhow::{Context, bail};
use check::{UpdateOutcome, cache_marker_path, perform_update, start_automatic_with};
use std::env;
use std::time::SystemTime;

const INSTALLER_URL: &str =
    "https://github.com/maty-millien/ggx/releases/latest/download/ggx-installer.sh";

pub fn start_automatic() {
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let Some(marker) = cache_marker_path(env::var_os("XDG_CACHE_HOME"), env::var_os("HOME")) else {
        return;
    };

    let updater = executable.with_file_name("ggx-update");
    let _ = start_automatic_with(
        &updater,
        &marker,
        SystemTime::now(),
        env::var_os("CI").is_some(),
    );
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

    let marker = cache_marker_path(env::var_os("XDG_CACHE_HOME"), env::var_os("HOME"));
    let (outcome, elapsed) = tui::timed_spinner("Checking for updates", || {
        perform_update(&updater, &executable, marker.as_deref(), current_version)
    })?;

    match outcome {
        UpdateOutcome::Current => {
            tui::step("Update check complete", elapsed);
            tui::warning("Already up to date");
        }
        UpdateOutcome::Updated(version) => {
            tui::success("New version", &version);
            tui::rail();
            tui::success("Update complete", &format!("{:.1}s", elapsed.as_secs_f32()));
        }
    }

    Ok(())
}
