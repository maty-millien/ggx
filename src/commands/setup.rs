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

            let current = config::load().ok();
            let mut providers = Provider::value_variants().to_vec();
            providers.sort_by_key(|provider| Some(*provider) != current);
            let mut choices: Vec<_> = providers
                .into_iter()
                .map(|provider| Choice::new(auth_config(provider), Some(provider)))
                .collect();
            choices.push(Choice::new("Cancel", None));

            let Some(provider) = tui::select(
                "Choose which auth config on this machine ggx uses",
                &choices,
            )?
            else {
                tui::aborted();
                return Ok(());
            };
            provider
        }
    };

    ai::validate(provider)?;
    config::save(provider)?;
    tui::success("ggx now uses your", auth_config(provider));

    Ok(())
}

fn auth_config(provider: Provider) -> &'static str {
    match provider {
        Provider::Codex => "Codex CLI auth config",
        Provider::Claude => "Claude Code auth config",
        Provider::Copilot => "GitHub Copilot auth config",
    }
}
