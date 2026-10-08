use crate::ai::{self, Provider};
use crate::config;
use crate::tui::{self, Choice};
use clap::ValueEnum;
use std::io::{self, IsTerminal};

pub fn run(provider: Option<Provider>) -> anyhow::Result<()> {
    let provider = match provider {
        Some(provider) => provider,
        None => {
            anyhow::ensure!(
                io::stdin().is_terminal() && io::stdout().is_terminal(),
                "`ggx setup` requires an interactive terminal."
            );

            // The current provider comes first, so Enter keeps it.
            let current = config::load().ok();
            let mut providers = Provider::value_variants().to_vec();
            providers.sort_by_key(|provider| Some(*provider) != current);
            let mut choices: Vec<_> = providers
                .into_iter()
                .map(|provider| Choice::new(provider.label(), Some(provider)))
                .collect();
            choices.push(Choice::new("Cancel", None));

            let Some(provider) = tui::select("Choose an AI provider", &choices)? else {
                tui::aborted();
                return Ok(());
            };
            provider
        }
    };

    ai::validate(provider)?;
    config::save(provider)?;
    tui::success("AI provider set to", provider.label());

    Ok(())
}
