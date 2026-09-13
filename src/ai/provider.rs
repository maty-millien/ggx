use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Codex,
    Claude,
    Copilot,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Copilot => "copilot",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::Copilot => "Copilot",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            "copilot" => Some(Self::Copilot),
            _ => None,
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;

    #[test]
    fn provider_names_round_trip() {
        for provider in [Provider::Codex, Provider::Claude, Provider::Copilot] {
            assert_eq!(Provider::parse(provider.as_str()), Some(provider));
            assert_eq!(provider.to_string(), provider.label());
        }
        assert_eq!(Provider::parse("other"), None);
        assert_eq!(Provider::parse("Codex"), None);
    }
}
