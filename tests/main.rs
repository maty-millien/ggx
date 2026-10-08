mod branch;
mod cli;
mod commit;
mod merge;
mod pr;
mod providers;
mod setup;
mod sync;
mod update;

use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CODEX_TOKEN: &str = "header.eyJleHAiOjQxMDI0NDQ4MDB9.signature";

const FAKE_TOOLS: &[&str] = &["gh", "curl", "security"];

const FAKE_TOOL: &str = r#"dir="$GGX_E2E_FAKES/$(basename "$0")"
mkdir -p "$dir"
n=$(( $(cat "$dir/count" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/count"
for arg in "$@"; do printf '%s\0' "$arg"; done > "$dir/$n.args"
if [ ! -f "$dir/$n.code" ]; then
  echo "unexpected $(basename "$0") call $n: $*" >&2
  exit 99
fi
cat "$dir/$n.out"
cat "$dir/$n.err" >&2
exit "$(cat "$dir/$n.code")""#;

const CLEARED_VARIABLES: &[&str] = &[
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "CODEX_HOME",
    "CLAUDE_CONFIG_DIR",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "CI",
    "CLICOLOR_FORCE",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
];

pub struct Env {
    pub root: PathBuf,
    pub home: PathBuf,
    pub repo: PathBuf,
    pub bin: PathBuf,
    fakes: PathBuf,
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Env {
    pub fn empty() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ggx-e2e-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);

        let env = Self {
            home: root.join("home"),
            repo: root.join("repo"),
            bin: root.join("bin"),
            fakes: root.join("fakes"),
            root,
        };
        for directory in [&env.home, &env.repo, &env.bin, &env.fakes] {
            fs::create_dir_all(directory).unwrap();
        }
        env.write_file(
            &env.home.join(".gitconfig"),
            "[user]\n\tname = Test User\n\temail = test@example.com\n[init]\n\tdefaultBranch = main\n",
        );
        for tool in FAKE_TOOLS {
            env.script(&env.bin.join(tool), FAKE_TOOL);
        }
        env
    }

    pub fn configured() -> Self {
        let env = Self::empty();
        env.write_file(&env.config_path(), r#"{"provider":"codex"}"#);
        env.sign_in_to_codex(CODEX_TOKEN);
        env
    }

    pub fn sign_in_to_codex(&self, token: &str) {
        self.write_file(
            &self.home.join(".codex/auth.json"),
            &format!(r#"{{"tokens":{{"access_token":"{token}","account_id":"account-1"}}}}"#),
        );
    }

    pub fn repo() -> Self {
        let env = Self::configured();
        env.git(&["init"]);
        env.write("README.md", "# Demo\n");
        env.commit_all("initial");
        env.git_in(&env.root, &["init", "--bare", "remote.git"]);
        env.git(&["remote", "add", "origin", "../remote.git"]);
        env.git(&["push", "-u", "origin", "main"]);
        env
    }

    pub fn config_path(&self) -> PathBuf {
        self.home.join(".config/ggx/config.json")
    }

    pub fn use_provider(&self, provider: &str) {
        self.write_file(
            &self.config_path(),
            &format!(r#"{{"provider":"{provider}"}}"#),
        );
    }

    pub fn command(&self, args: &[&str]) -> Command {
        self.command_for(Path::new(env!("CARGO_BIN_EXE_ggx")), args)
    }

    pub fn command_for(&self, executable: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(executable);
        command.args(args);
        self.isolate(&mut command);
        command
    }

    pub fn run(&self, args: &[&str]) -> Run {
        self.output(self.command(args), "")
    }

    pub fn run_with_input(&self, args: &[&str], input: &str) -> Run {
        self.output(self.command(args), input)
    }

    pub fn output(&self, mut command: Command, input: &str) -> Run {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
        let output = child.wait_with_output().unwrap();

        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    pub fn respond(&self, tool: &str, stdout: &str) {
        self.respond_with(tool, stdout, "", 0);
    }

    pub fn respond_with(&self, tool: &str, stdout: &str, stderr: &str, code: i32) {
        let directory = self.fakes.join(tool);
        fs::create_dir_all(&directory).unwrap();
        let call = (1..)
            .find(|call| !directory.join(format!("{call}.code")).exists())
            .unwrap();
        fs::write(directory.join(format!("{call}.out")), stdout).unwrap();
        fs::write(directory.join(format!("{call}.err")), stderr).unwrap();
        fs::write(directory.join(format!("{call}.code")), code.to_string()).unwrap();
    }

    pub fn calls(&self, tool: &str) -> Vec<Vec<String>> {
        let directory = self.fakes.join(tool);
        (1..)
            .map(|call| directory.join(format!("{call}.args")))
            .take_while(|path| path.exists())
            .map(|path| {
                fs::read_to_string(path)
                    .unwrap()
                    .split_terminator('\0')
                    .map(str::to_string)
                    .collect()
            })
            .collect()
    }

    pub fn reply(&self, output: Value) {
        self.reply_text(&output.to_string());
    }

    pub fn reply_text(&self, text: &str) {
        self.respond("curl", &codex_stream(text));
    }

    pub fn prompts(&self) -> Vec<String> {
        self.calls("curl")
            .iter()
            .map(|args| {
                request_body(args)["input"][0]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.repo, args)
    }

    pub fn git_in(&self, directory: &Path, args: &[&str]) -> String {
        let output = self.git_command(directory, args).output().unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub fn git_status(&self, args: &[&str]) -> bool {
        self.git_command(&self.repo, args)
            .output()
            .unwrap()
            .status
            .success()
    }

    fn git_command(&self, directory: &Path, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command.args(args);
        self.isolate(&mut command);
        command.current_dir(directory);
        command
    }

    pub fn branch(&self) -> String {
        self.git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .trim()
            .to_string()
    }

    pub fn branches(&self) -> String {
        self.git(&["branch", "--format", "%(refname:short)"])
    }

    pub fn remote_branches(&self) -> String {
        self.git_in(
            &self.root.join("remote.git"),
            &["branch", "--format", "%(refname:short)"],
        )
    }

    pub fn last_commit(&self) -> String {
        self.git(&["log", "-1", "--format=%s"]).trim().to_string()
    }

    pub fn write(&self, relative: &str, contents: &str) {
        self.write_file(&self.repo.join(relative), contents);
    }

    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "-m", message]);
    }

    pub fn write_file(&self, path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub fn script(&self, path: &Path, body: &str) {
        let source = self.root.join(format!(
            "{}.src",
            path.file_name().unwrap().to_string_lossy()
        ));
        self.write_file(&source, &format!("#!/bin/sh\n{body}\n"));
        self.install(&source, path);
    }

    pub fn install(&self, source: &Path, destination: &Path) {
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        let status = Command::new("install")
            .args(["-m", "755"])
            .arg(source)
            .arg(destination)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn isolate(&self, command: &mut Command) {
        for name in CLEARED_VARIABLES {
            command.env_remove(name);
        }
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .current_dir(&self.repo)
            .env("HOME", &self.home)
            .env("PATH", path)
            .env("GGX_E2E_FAKES", &self.fakes)
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Run {
    #[track_caller]
    pub fn success(&self) -> &str {
        assert_eq!(
            self.code, 0,
            "stdout:\n{}\nstderr:\n{}",
            self.stdout, self.stderr
        );
        &self.stdout
    }

    #[track_caller]
    pub fn failure(&self, message: &str) {
        assert_eq!(
            (self.code, self.stderr.as_str()),
            (1, format!("+ {message}\n").as_str()),
            "stdout:\n{}",
            self.stdout
        );
    }
}

pub fn generated(
    branch: Option<&str>,
    commit: Option<&str>,
    pull_request: Option<(&str, &str)>,
) -> Value {
    json!({
        "branch": branch,
        "commit": commit,
        "pull_request": pull_request.map(|(title, body)| json!({"title": title, "body": body})),
    })
}

pub fn codex_stream(text: &str) -> String {
    let middle = text
        .char_indices()
        .nth(text.chars().count() / 2)
        .map_or(0, |(index, _)| index);
    let (first, second) = text.split_at(middle);
    [
        json!({"type": "response.created"}),
        json!({"type": "response.output_text.delta", "delta": first}),
        json!({"type": "response.output_text.delta", "delta": second}),
        json!({"type": "response.completed"}),
    ]
    .iter()
    .map(|event| format!("event: message\ndata: {event}\n\n"))
    .collect()
}

pub fn request_body(args: &[String]) -> Value {
    serde_json::from_str(argument_after(args, "--data-binary")).unwrap()
}

pub fn argument_after<'a>(args: &'a [String], flag: &str) -> &'a str {
    let index = args.iter().position(|arg| arg == flag).unwrap();
    &args[index + 1]
}

pub fn has_header(args: &[String], header: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == "-H" && pair[1] == header)
}

pub fn section<'a>(prompt: &'a str, heading: &str) -> &'a str {
    let start = prompt
        .find(heading)
        .unwrap_or_else(|| panic!("missing {heading} in:\n{prompt}"));
    let body = &prompt[start..];
    let body = &body[body.find("\n\x60\x60\x60\x60").unwrap() + 5..];
    let body = &body[body.find('\n').unwrap() + 1..];
    &body[..body.find("\n\x60\x60\x60\x60").unwrap()]
}
