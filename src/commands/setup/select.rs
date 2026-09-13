use crate::ai::Provider;

pub(super) fn provider_options(current: Option<Provider>) -> Vec<(&'static str, Option<Provider>)> {
    let mut providers = vec![Provider::Codex, Provider::Claude, Provider::Copilot];
    if let Some(current) = current {
        providers.retain(|provider| *provider != current);
        providers.insert(0, current);
    }

    let mut options = providers
        .into_iter()
        .map(|provider| (provider.label(), Some(provider)))
        .collect::<Vec<_>>();
    options.push(("Cancel", None));
    options
}

pub(super) fn complete<V, S>(
    selected: Option<Provider>,
    mut validate: V,
    mut save: S,
) -> anyhow::Result<bool>
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
                ("Copilot", Some(Provider::Copilot)),
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
                ("Copilot", Some(Provider::Copilot)),
                ("Cancel", None)
            ]
        );
    }

    #[test]
    fn validates_then_saves_selected_provider() {
        let saved = Cell::new(None);
        let result = complete(
            Some(Provider::Codex),
            |_| Ok(()),
            |provider| {
                saved.set(Some(provider));
                Ok(())
            },
        );

        assert!(result.unwrap());
        assert_eq!(saved.get(), Some(Provider::Codex));
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
