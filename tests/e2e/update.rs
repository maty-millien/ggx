use crate::support::{Env, VERSION};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const REPLACE_GGX: &str = r#"cd "$(dirname "$0")"
printf '#!/bin/sh\n%s\n' "$GGX_REPLACEMENT" > ggx.new
chmod 755 ggx.new
mv ggx.new ggx"#;

/// Installs a copy of ggx next to a fake ggx-update script.
fn install(env: &Env, updater: &str) -> PathBuf {
    let directory = env.root.join("install");
    let executable = directory.join("ggx");
    env.install(Path::new(env!("CARGO_BIN_EXE_ggx")), &executable);
    env.script(&directory.join("ggx-update"), updater);
    executable
}

fn update(env: &Env, executable: &Path, replacement: &str) -> crate::support::Run {
    let mut command = env.command_for(executable, &["update"]);
    command.env("GGX_REPLACEMENT", replacement);
    env.output(command, "")
}

#[test]
fn reports_a_missing_updater() {
    Env::configured().run(&["update"]).failure(
        "ggx-update is missing; reinstall ggx with the official installer: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/maty-millien/ggx/releases/latest/download/ggx-installer.sh | sh",
    );
}

#[test]
fn installs_a_new_version() {
    let env = Env::configured();
    let executable = install(&env, REPLACE_GGX);
    let cache = env.root.join("cache");
    let mut command = env.command_for(&executable, &["update"]);
    command
        .env("GGX_REPLACEMENT", "echo 'ggx 9.9.9'")
        .env("XDG_CACHE_HOME", &cache);

    let stdout = env.output(command, "").success().to_string();

    assert!(
        stdout.starts_with(&format!("+ Current version {VERSION}\n│\n")),
        "{stdout}"
    );
    assert!(stdout.contains("+ New version 9.9.9\n│\n+ Update complete "));
    assert!(cache.join("ggx/update-check").is_file());
}

#[test]
fn reports_when_already_up_to_date() {
    let env = Env::configured();
    let executable = install(&env, "exit 0");

    let stdout = update(&env, &executable, "").success().to_string();

    assert!(stdout.ends_with("+ Already up to date\n"), "{stdout}");
    assert!(env.home.join(".cache/ggx/update-check").is_file());
}

#[test]
fn reports_updater_failures() {
    let cases = [
        (
            "echo boom >&2; echo out; exit 1",
            "",
            "ggx update failed: boom",
        ),
        ("echo out; exit 1", "", "ggx update failed: out"),
        ("exit 4", "", "ggx update failed with exit status: 4"),
        (
            REPLACE_GGX,
            "echo nonsense",
            "ggx returned an invalid version",
        ),
        (
            REPLACE_GGX,
            "exit 1",
            "Could not read the installed ggx version",
        ),
    ];

    for (updater, replacement, error) in cases {
        let env = Env::configured();
        let executable = install(&env, updater);
        update(&env, &executable, replacement).failure(error);
    }
}

#[test]
fn checks_for_updates_in_the_background_once_a_day() {
    let env = Env::configured();
    let executable = install(&env, r#"echo ran >> "$(dirname "$0")/runs""#);
    let runs = env.root.join("install/runs");
    let marker = env.home.join(".cache/ggx/update-check");
    let run_sync = |ci: bool| {
        let mut command = env.command_for(&executable, &["sync"]);
        if ci {
            command.env("CI", "true");
        }
        // sync fails outside a repository, after the update check started.
        assert_eq!(env.output(command, "").code, 1);
    };

    run_sync(false);
    wait_for_runs(&runs, 1);
    assert!(marker.is_file());

    run_sync(false);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(count_runs(&runs), 1);

    let yesterday = SystemTime::now() - Duration::from_secs(25 * 60 * 60);
    File::options()
        .write(true)
        .open(&marker)
        .unwrap()
        .set_modified(yesterday)
        .unwrap();
    run_sync(true);
    thread::sleep(Duration::from_millis(300));
    assert_eq!(count_runs(&runs), 1);

    run_sync(false);
    wait_for_runs(&runs, 2);
}

fn count_runs(runs: &Path) -> usize {
    fs::read_to_string(runs).map_or(0, |runs| runs.lines().count())
}

fn wait_for_runs(runs: &Path, expected: usize) {
    let started = Instant::now();
    while count_runs(runs) < expected {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "updater did not run"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
