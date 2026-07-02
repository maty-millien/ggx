use std::env;
use std::process::{Command, Stdio};

const BREW_UPDATE_COMMAND: &str = "brew update && brew upgrade ggx";

pub fn brew_update() {
    if should_skip(env::var_os("CI").is_some(), brew_exists()) {
        return;
    }

    let _ = update_command().spawn();
}

fn should_skip(ci: bool, brew_available: bool) -> bool {
    ci || !brew_available
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
        .arg(BREW_UPDATE_COMMAND)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(test)]
mod tests {
    use super::should_skip;

    #[test]
    fn should_skip_in_ci() {
        assert!(should_skip(true, true));
    }

    #[test]
    fn should_skip_without_brew() {
        assert!(should_skip(false, false));
    }

    #[test]
    fn should_not_skip_outside_ci_with_brew() {
        assert!(!should_skip(false, true));
    }
}
