mod select;

use crate::ai;
use crate::config;
use crate::tui::{self, Choice};
use select::{complete, provider_options};
use std::io::{self, IsTerminal};

pub fn run() -> anyhow::Result<()> {
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "`ggx setup` requires an interactive terminal."
    );

    let options = provider_options(config::current());
    let choices = options
        .iter()
        .map(|&(label, provider)| Choice::new(label, provider))
        .collect::<Vec<_>>();
    let selected = tui::select("Choose an AI provider", &choices)?;

    if complete(selected, ai::validate, config::save)? {
        tui::success(
            "AI provider set to",
            selected.expect("provider was selected").label(),
        );
    } else {
        tui::aborted();
    }

    Ok(())
}
