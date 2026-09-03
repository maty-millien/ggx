use crate::ai::{self, Provider};
use crate::config;
use crate::tui::{self, Choice};
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

fn provider_options(current: Option<Provider>) -> Vec<(&'static str, Option<Provider>)> {
    let mut providers = match current {
        Some(Provider::Claude) => vec![
            ("Claude", Some(Provider::Claude)),
            ("Codex", Some(Provider::Codex)),
        ],
        Some(Provider::Codex) | None => vec![
            ("Codex", Some(Provider::Codex)),
            ("Claude", Some(Provider::Claude)),
        ],
    };
    providers.push(("Cancel", None));
    providers
}

fn complete<V, S>(selected: Option<Provider>, mut validate: V, mut save: S) -> anyhow::Result<bool>
where
    V: FnMut(Provider) -> anyhow::Result<()>,
    S: FnMut(Provider) -> anyhow::Result<()>,
{
    let Some(provider) = selected else {
        return Ok(false);
    };

    validate(provider)?;
    save(provider)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{complete, provider_options};
    use crate::ai::Provider;
    use std::cell::Cell;

    #[test]
    fn lists_codex_first_without_configuration() {
        assert_eq!(
            provider_options(None),
            vec![
                ("Codex", Some(Provider::Codex)),
                ("Claude", Some(Provider::Claude)),
                ("Cancel", None)
            ]
        );
    }

    #[test]
    fn lists_current_provider_first() {
        assert_eq!(
            provider_options(Some(Provider::Claude)),
            vec![
                ("Claude", Some(Provider::Claude)),
                ("Codex", Some(Provider::Codex)),
                ("Cancel", None)
            ]
        );
    }

    #[test]
    fn validates_before_saving() {
        let saved = Cell::new(false);
        let result = complete(
            Some(Provider::Claude),
            |_| anyhow::bail!("missing Claude"),
            |_| {
                saved.set(true);
                Ok(())
            },
        );

        assert_eq!(result.unwrap_err().to_string(), "missing Claude");
        assert!(!saved.get());
    }

    #[test]
    fn cancellation_does_not_validate_or_save() {
        let validated = Cell::new(false);
        let saved = Cell::new(false);
        let result = complete(
            None,
            |_| {
                validated.set(true);
                Ok(())
            },
            |_| {
                saved.set(true);
                Ok(())
            },
        );

        assert!(!result.unwrap());
        assert!(!validated.get());
        assert!(!saved.get());
    }
}
