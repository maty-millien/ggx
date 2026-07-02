use std::env;
use std::process::{Command, Stdio};

pub fn brew() {
    if env::var_os("CI").is_some() || !brew_exists() {
        return;
    }

    let _ = update_command().spawn();
}

fn brew_exists() -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|path| path.join("brew").is_file())
}

fn update_command() -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("brew update && brew upgrade ggx")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
