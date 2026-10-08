use crate::support::{Env, VERSION};
use std::fs;

#[test]
fn prints_help_without_a_command() {
    let run = Env::empty().run(&[]);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("Usage: ggx"), "{}", run.stderr);
}

#[test]
fn prints_the_version() {
    let env = Env::empty();

    for flag in ["--version", "-v"] {
        assert_eq!(env.run(&[flag]).success(), format!("ggx {VERSION}\n"));
    }
}

#[test]
fn commands_require_setup() {
    Env::empty()
        .run(&["commit"])
        .failure("ggx is not set up. Run `ggx setup` to choose an AI provider.");
}

#[test]
fn rejects_invalid_configuration() {
    let env = Env::empty();
    let message = format!(
        "Invalid ggx configuration at {}. Run `ggx setup` again.",
        env.config_path().display()
    );

    for contents in ["not json", "{}", r#"{"provider":"gpt"}"#] {
        env.write_file(&env.config_path(), contents);
        env.run(&["commit"]).failure(&message);
    }
}

#[test]
fn reports_unreadable_configuration() {
    let env = Env::empty();
    fs::create_dir_all(env.config_path()).unwrap();

    let run = env.run(&["commit"]);

    assert_eq!(run.code, 1);
    let prefix = format!(
        "+ Could not read ggx configuration at {}: ",
        env.config_path().display()
    );
    assert!(run.stderr.starts_with(&prefix), "{}", run.stderr);
    assert!(run.stderr.ends_with(". Run `ggx setup` again.\n"));
}

#[test]
fn requires_a_configuration_directory() {
    let env = Env::empty();
    let mut command = env.command(&["commit"]);
    command.env_remove("HOME");

    env.output(command, "").failure(
        "Could not locate the user configuration directory. Set XDG_CONFIG_HOME or HOME, then run `ggx setup`.",
    );
}

#[test]
fn reads_configuration_from_xdg_config_home() {
    let env = Env::empty();
    let xdg = env.root.join("xdg");
    env.write_file(&xdg.join("ggx/config.json"), r#"{"provider":"codex"}"#);

    let mut command = env.command(&["commit"]);
    command.env("XDG_CONFIG_HOME", &xdg);
    let run = env.output(command, "");

    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .starts_with("+ git ls-files --unmerged failed: fatal: not a git repository"),
        "{}",
        run.stderr
    );
}

#[test]
fn ignores_an_empty_xdg_config_home() {
    let env = Env::configured();
    let mut command = env.command(&["commit"]);
    command.env("XDG_CONFIG_HOME", "");

    let run = env.output(command, "");

    assert!(
        run.stderr.contains("not a git repository"),
        "{}",
        run.stderr
    );
}
