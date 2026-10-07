use crate::vcs::github;

fn value_or_unknown(value: &str) -> &str {
    if value.is_empty() { "unknown" } else { value }
}

fn readable_status(value: &str) -> String {
    let value = value_or_unknown(value).replace('_', " ").to_lowercase();
    let mut characters = value.chars();

    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

pub(super) fn summary(pull_request: &github::PullRequest) -> String {
    let mut lines = vec![
        format!("#{}  {}", pull_request.number, pull_request.title),
        pull_request.url.clone(),
        String::new(),
        format!(
            "{:<12}{} → {}",
            "Branch", pull_request.head, pull_request.base
        ),
        format!(
            "{:<12}{}",
            "Merge state",
            readable_status(&pull_request.merge_state)
        ),
    ];

    if !pull_request.review_decision.is_empty() {
        lines.push(format!(
            "{:<12}{}",
            "Review",
            readable_status(&pull_request.review_decision)
        ));
    }

    lines.join("\n")
}
