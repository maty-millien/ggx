use crate::setup::{copilot_token, sign_in_to_copilot};
use crate::support::{Env, generated, has_header, request_body};
use serde_json::{Value, json};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const COMMIT: &str = "feat(core): add change";
const CLAUDE_NOT_SIGNED_IN: &str =
    "Claude is not signed in or the token has expired. Run 'claude login' first.";

fn pending_change(env: &Env, name: &str) {
    env.write(name, "change\n");
}

fn output() -> Value {
    generated(None, Some(COMMIT), None)
}

fn claude_response(blocks: Value) -> String {
    json!({"content": blocks}).to_string()
}

fn claude_credentials(expires_in_ms: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    json!({"claudeAiOauth": {"accessToken": "oauth-token", "expiresAt": now + expires_in_ms}})
        .to_string()
}

#[test]
fn codex_sends_the_prompt_with_the_cli_login() {
    let env = Env::repo();
    pending_change(&env, "a.txt");
    env.reply(output());

    env.run(&["commit", "-y"]).success();

    let call = &env.calls("curl")[0];
    assert_eq!(
        call.last().unwrap(),
        "https://chatgpt.com/backend-api/codex/responses"
    );
    assert!(has_header(call, "Authorization: Bearer codex-token"));
    assert!(has_header(call, "chatgpt-account-id: account-1"));
    let body = request_body(call);
    assert_eq!(body["model"], "gpt-6-luna");
    assert_eq!(body["reasoning"]["effort"], "none");
    assert_eq!(body["stream"], true);
    assert_eq!(body["store"], false);
    assert_eq!(env.last_commit(), COMMIT);
}

#[test]
fn codex_reads_the_login_from_codex_home() {
    let env = Env::repo();
    let codex_home = env.root.join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    fs::rename(
        env.home.join(".codex/auth.json"),
        codex_home.join("auth.json"),
    )
    .unwrap();
    pending_change(&env, "a.txt");
    env.reply(output());

    let mut command = env.command(&["commit", "-y"]);
    command.env("CODEX_HOME", &codex_home);
    env.output(command, "").success();

    assert_eq!(env.last_commit(), COMMIT);
}

#[test]
fn codex_requires_a_login() {
    let env = Env::repo();
    pending_change(&env, "a.txt");
    let message = "Codex is not signed in. Run 'codex login' to refresh the token in auth.json.";

    for auth in ["not json", r#"{"tokens":{"access_token":"token"}}"#] {
        env.write_file(&env.home.join(".codex/auth.json"), auth);
        env.run(&["commit", "-y"]).failure(message);
    }
    fs::remove_file(env.home.join(".codex/auth.json")).unwrap();
    env.run(&["commit", "-y"]).failure(message);
    assert!(env.calls("curl").is_empty());
}

#[test]
fn codex_reports_failures() {
    let env = Env::repo();
    pending_change(&env, "a.txt");

    for _ in 0..2 {
        env.respond_with(
            "curl",
            r#"{"detail":"Unauthorized"}"#,
            "curl: (22) The requested URL returned error: 401\n",
            22,
        );
    }
    env.run(&["commit", "-y"]).failure(
        r#"Codex request failed: curl: (22) The requested URL returned error: 401 {"detail":"Unauthorized"}"#,
    );

    for _ in 0..2 {
        env.respond("curl", "data: {\"type\":\"response.completed\"}\n\n");
    }
    env.run(&["commit", "-y"])
        .failure("Codex returned an empty response");
}

#[test]
fn claude_uses_the_proxy_token_and_base_url() {
    let env = Env::repo();
    env.use_provider("claude");
    pending_change(&env, "a.txt");
    let text = output().to_string();
    let (first, second) = text.split_at(10);
    env.respond(
        "curl",
        &claude_response(json!([
            {"type": "thinking", "thinking": "ignored"},
            {"type": "text", "text": first},
            {"type": "text", "text": second},
        ])),
    );

    let mut command = env.command(&["commit", "-y"]);
    command
        .env("ANTHROPIC_BASE_URL", "https://proxy.example")
        .env("ANTHROPIC_AUTH_TOKEN", "proxy-token");
    env.output(command, "").success();

    let call = &env.calls("curl")[0];
    assert_eq!(call.last().unwrap(), "https://proxy.example/v1/messages");
    assert!(has_header(call, "Authorization: Bearer proxy-token"));
    assert!(!has_header(call, "anthropic-beta: oauth-2025-04-20"));
    let body = request_body(call);
    assert_eq!(body["model"], "claude-haiku-5-5");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(
        body["system"],
        "You are Claude Code, Anthropic's official CLI for Claude."
    );
    assert!(
        body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .starts_with("Do not invoke tools.\n")
    );
    assert_eq!(env.last_commit(), COMMIT);
}

#[test]
fn claude_uses_the_cli_oauth_credentials() {
    let env = Env::repo();
    env.use_provider("claude");
    env.write_file(
        &env.home.join(".claude/.credentials.json"),
        &claude_credentials(3_600_000),
    );
    pending_change(&env, "a.txt");
    env.respond(
        "curl",
        &claude_response(json!([{"type": "text", "text": output().to_string()}])),
    );

    env.run(&["commit", "-y"]).success();

    let call = &env.calls("curl")[0];
    assert_eq!(
        call.last().unwrap(),
        "https://api.anthropic.com/v1/messages"
    );
    assert!(has_header(call, "Authorization: Bearer oauth-token"));
    assert!(has_header(call, "anthropic-beta: oauth-2025-04-20"));

    let config = env.root.join("claude-config");
    fs::create_dir_all(&config).unwrap();
    fs::rename(
        env.home.join(".claude/.credentials.json"),
        config.join(".credentials.json"),
    )
    .unwrap();
    pending_change(&env, "b.txt");
    env.respond(
        "curl",
        &claude_response(json!([{"type": "text", "text": output().to_string()}])),
    );
    let mut command = env.command(&["commit", "-y"]);
    command.env("CLAUDE_CONFIG_DIR", &config);
    env.output(command, "").success();
}

#[test]
fn claude_requires_a_valid_login() {
    let env = Env::repo();
    env.use_provider("claude");
    pending_change(&env, "a.txt");

    env.run(&["commit", "-y"]).failure(CLAUDE_NOT_SIGNED_IN);

    env.write_file(
        &env.home.join(".claude/.credentials.json"),
        &claude_credentials(30_000),
    );
    env.run(&["commit", "-y"]).failure(CLAUDE_NOT_SIGNED_IN);
    assert!(env.calls("curl").is_empty());
}

#[test]
fn claude_reads_the_macos_keychain() {
    if !cfg!(target_os = "macos") {
        return;
    }
    let env = Env::repo();
    env.use_provider("claude");
    pending_change(&env, "a.txt");
    env.respond("security", &claude_credentials(3_600_000));
    env.respond(
        "curl",
        &claude_response(json!([{"type": "text", "text": output().to_string()}])),
    );

    env.run(&["commit", "-y"]).success();

    assert_eq!(
        env.calls("security")[0],
        [
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w"
        ]
    );
    assert!(has_header(
        &env.calls("curl")[0],
        "Authorization: Bearer oauth-token"
    ));
}

#[test]
fn claude_reports_failures() {
    let env = Env::repo();
    env.use_provider("claude");
    pending_change(&env, "a.txt");
    let cases: [(&str, &str, i32, &str); 3] = [
        (
            r#"{"error":"overloaded"}"#,
            "curl: (22) The requested URL returned error: 529\n",
            22,
            r#"Claude request failed: curl: (22) The requested URL returned error: 529 {"error":"overloaded"}"#,
        ),
        ("not json", "", 0, "Claude returned an unexpected response"),
        (
            r#"{"content":[]}"#,
            "",
            0,
            "Claude returned an empty response",
        ),
    ];

    for (stdout, stderr, code, error) in cases {
        env.respond_with("curl", stdout, stderr, code);
        env.respond_with("curl", stdout, stderr, code);
        let mut command = env.command(&["commit", "-y"]);
        command.env("ANTHROPIC_AUTH_TOKEN", "proxy-token");
        env.output(command, "").failure(error);
    }
}

fn copilot_completion(chunks: &[&str]) -> String {
    let mut events: String = chunks
        .iter()
        .map(|chunk| format!("data: {}\n\n", json!({"choices": [{"text": chunk}]})))
        .collect();
    events.push_str("data: [DONE]\n\n");
    events
}

#[test]
fn copilot_completes_with_a_cached_session() {
    let env = Env::repo();
    env.use_provider("copilot");
    sign_in_to_copilot(&env, "apps.json");
    let completion = copilot_completion(&[
        " null, \"commit\": \"feat(core): add change\",",
        " \"pull_request\": null}\n\nThe JSON above {\"extra\": 1}",
    ]);
    env.respond("curl", &copilot_token(3600));
    env.respond("curl", &completion);
    pending_change(&env, "a.txt");

    env.run(&["commit", "-y"]).success();

    let calls = env.calls("curl");
    assert_eq!(
        calls[1].last().unwrap(),
        "https://proxy.example/v1/engines/copilot-codex/completions"
    );
    assert!(has_header(&calls[1], "Authorization: Bearer tid=copilot"));
    let body = request_body(&calls[1]);
    assert!(
        body["prompt"]
            .as_str()
            .unwrap()
            .ends_with("\n\n## Response\n\n{\"branch\":")
    );
    assert_eq!(env.last_commit(), COMMIT);

    env.respond("curl", &completion);
    pending_change(&env, "b.txt");
    env.run(&["commit", "-y"]).success();
    assert_eq!(env.calls("curl").len(), 3);

    env.write_file(
        &env.home.join(".config/ggx/copilot-token.json"),
        &copilot_token(30),
    );
    env.respond("curl", &copilot_token(3600));
    env.respond("curl", &completion);
    pending_change(&env, "c.txt");
    env.run(&["commit", "-y"]).success();
    assert_eq!(env.calls("curl").len(), 5);
}

#[test]
fn copilot_reports_token_failures() {
    let env = Env::repo();
    env.use_provider("copilot");
    sign_in_to_copilot(&env, "apps.json");
    pending_change(&env, "a.txt");

    for _ in 0..2 {
        env.respond_with(
            "curl",
            "",
            "curl: (22) The requested URL returned error: 401\n",
            22,
        );
    }
    env.run(&["commit", "-y"])
        .failure("Could not obtain a Copilot token");

    for _ in 0..2 {
        env.respond("curl", "{}");
    }
    env.run(&["commit", "-y"])
        .failure("Copilot returned an unexpected token response");
}
