use crate::commands::pr::{changes, context::Context, git, github, prompt};
use crate::{ai, tui};
use std::time::Instant;

pub fn run(draft: bool, base: Option<String>, closes: Vec<String>) -> anyhow::Result<()> {
    let started = Instant::now();
    let upstream = git::upstream()?;
    let context = Context::collect(base, closes)?;

    tui::step("Analysis complete", started.elapsed());
    tui::section("Changes");
    tui::change_rows(&changes::from_context(&context));

    let (generated, elapsed) = tui::timed_spinner("Generating pull request", || {
        ai::generate(&prompt::render(&context))
    })?;
    let pull_request = PullRequest::parse(&generated)?;

    tui::step("Pull request generated", elapsed);
    tui::section("Title");
    tui::message(&pull_request.title);
    tui::section("Body");
    tui::block(&pull_request.body);

    let confirm_prompt = format!("Sync {} and create PR into {}?", upstream, context.base);

    if tui::confirm(&confirm_prompt)? {
        git::push(&upstream)?;
        tui::rail();
        let url = tui::spinner("Creating pull request", || {
            github::create_pr(
                &context.base,
                &context.branch,
                &pull_request.title,
                &pull_request.body,
                draft,
            )
        })?;
        tui::success("Created PR", &url);
    } else {
        tui::warning("Aborted");
    }

    Ok(())
}

struct PullRequest {
    title: String,
    body: String,
}

impl PullRequest {
    fn parse(output: &str) -> anyhow::Result<Self> {
        let output = output.trim();
        let Some((title, body)) = output.split_once("\n\n") else {
            anyhow::bail!("Generated pull request must include a title, blank line, and body.");
        };

        let title = title.trim().to_string();
        let body = body.trim().to_string();

        if title.is_empty() || body.is_empty() {
            anyhow::bail!("Generated pull request title and body must not be empty.");
        }

        Ok(Self { title, body })
    }
}
