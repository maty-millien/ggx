use crate::{CODEX_TOKEN, Env};
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const COPILOT_NOT_SIGNED_IN: &str = "Copilot is not signed in. Sign in with the GitHub Copilot CLI or an editor with Copilot first.";

pub fn copilot_token(expires_in: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    json!({
        "token": "tid=copilot",
        "expires_at": now + expires_in,
        "endpoints": {"proxy": "https://proxy.example"},
    })
    .to_string()
}

pub fn sign_in_to_copilot(env: &Env, file: &str) {
    env.write_file(
        &env.home.join(".config/github-copilot").join(file),
        r#"{"github.com:Iv1.b507a08c87ecfe98":{"user":"octocat","oauth_token":"gho_editor"}}"#,
    );
}

#[test]
fn saves_a_validated_provider() {
    let env = Env::empty();
    env.sign_in_to_codex(CODEX_TOKEN);

    let run = env.run(&["setup", "--provider", "codex"]);

    assert_eq!(run.success(), "+ AI provider set to Codex\n");
    assert_eq!(
        fs::read_to_string(env.config_path()).unwrap(),
        "{\"provider\":\"codex\"}\n"
    );
}

#[test]
fn saves_under_xdg_config_home() {
    let env = Env::empty();
    let xdg = env.root.join("xdg");

    let mut command = env.command(&["setup", "--provider", "claude"]);
    command
        .env("XDG_CONFIG_HOME", &xdg)
        .env("ANTHROPIC_AUTH_TOKEN", "proxy-token");

    assert_eq!(
        env.output(command, "").success(),
        "+ AI provider set to Claude\n"
    );
    assert_eq!(
        fs::read_to_string(xdg.join("ggx/config.json")).unwrap(),
        "{\"provider\":\"claude\"}\n"
    );
    assert!(!env.config_path().exists());
}

#[test]
fn rejects_missing_or_expired_logins_without_saving() {
    let env = Env::empty();

    env.run(&["setup", "--provider", "codex"])
        .failure("Codex is not signed in or the token has expired. Run 'codex login' first.");
    env.sign_in_to_codex("header.eyJleHAiOjF9.signature");
    env.run(&["setup", "--provider", "codex"])
        .failure("Codex is not signed in or the token has expired. Run 'codex login' first.");
    env.run(&["setup", "--provider", "claude"])
        .failure("Claude is not signed in or the token has expired. Run 'claude login' first.");
    assert!(!env.config_path().exists());
}

#[test]
fn copilot_setup_exchanges_the_editor_login_for_a_token() {
    let env = Env::empty();
    sign_in_to_copilot(&env, "apps.json");
    let token = copilot_token(3600);
    env.respond("curl", &token);

    let run = env.run(&["setup", "--provider", "copilot"]);

    assert_eq!(run.success(), "+ AI provider set to Copilot\n");
    let call = &env.calls("curl")[0];
    assert!(call.contains(&"Authorization: token gho_editor".to_string()));
    assert_eq!(call.last().unwrap(), COPILOT_TOKEN_URL);
    assert_eq!(
        fs::read_to_string(env.home.join(".config/ggx/copilot-token.json")).unwrap(),
        token
    );
}

#[test]
fn copilot_setup_falls_back_to_hosts_json() {
    let env = Env::empty();
    sign_in_to_copilot(&env, "hosts.json");
    env.respond("curl", &copilot_token(3600));

    env.run(&["setup", "--provider", "copilot"]).success();

    assert!(env.calls("curl")[0].contains(&"Authorization: token gho_editor".to_string()));
}

#[test]
fn copilot_setup_requires_a_login() {
    let env = Env::empty();

    env.run(&["setup", "--provider", "copilot"])
        .failure(COPILOT_NOT_SIGNED_IN);
    assert!(env.calls("curl").is_empty());
}

#[test]
fn interactive_setup_requires_a_terminal() {
    Env::empty()
        .run(&["setup"])
        .failure("`ggx setup` requires an interactive terminal.");
}

#[test]
fn rejects_unknown_providers() {
    let run = Env::empty().run(&["setup", "--provider", "gpt"]);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("invalid value 'gpt'"), "{}", run.stderr);
}
