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
