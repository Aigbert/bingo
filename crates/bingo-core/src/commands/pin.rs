//! Model pins persist independently of the session's active choice.
use crate::host::{Host, pins};
use bingo_sdk::*;
use std::sync::Arc;

pub(super) fn parse(args: &str) -> Option<(bool, &str)> {
    let args = args.trim();
    let (verb, rest) = args.split_once(char::is_whitespace).unwrap_or((args, ""));
    match verb {
        "pin" => Some((true, rest.trim())),
        "unpin" => Some((false, rest.trim())),
        _ => None,
    }
}

pub(super) async fn run(
    host: &Arc<Host>,
    cx: &CommandContext,
    id: &str,
    pin: bool,
) -> Result<CommandOutcome, KernelError> {
    if id.is_empty() || id.chars().any(char::is_whitespace) || id.starts_with("--") {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "usage: /model pin|unpin <id>",
        ));
    }
    let entries = host.catalog(CatalogKind::Models).await?.entries;
    let current = host
        .session_model(&cx.session)
        .await?
        .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "a log session has no model"))?;
    let qualified = format!("{}/{id}", current.endpoint.id());
    let id = if entries.iter().any(|entry| entry.id == id)
        || pins::pinned_models(host).iter().any(|entry| entry == id)
    {
        id
    } else {
        &qualified
    };
    if pin && !entries.iter().any(|entry| entry.id == id) {
        return Err(unknown(id, &entries));
    }
    pins::pin_model(host, id, pin)?;
    Ok(CommandOutcome::Applied {
        message: Some(format!("{} {id}", if pin { "pinned" } else { "unpinned" })),
    })
}

fn unknown(id: &str, entries: &[CatalogEntry]) -> KernelError {
    let mut closest: Vec<_> = entries.iter().map(|entry| entry.id.as_str()).collect();
    closest.sort_by_key(|candidate| distance(candidate, id));
    KernelError::new(
        ErrorCode::InvalidInput,
        format!(
            "unknown model {id}; closest ids: {}",
            closest.into_iter().take(3).collect::<Vec<_>>().join(", ")
        ),
    )
}

fn distance(candidate: &str, wanted: &str) -> usize {
    let wanted: Vec<_> = wanted.chars().collect();
    let mut row: Vec<_> = (0..=wanted.len()).collect();
    for (i, a) in candidate.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, b) in wanted.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(a != *b));
            diagonal = above;
        }
    }
    row.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_ids_are_usage_and_suggestions_rank_by_spelling() {
        assert_eq!(parse("pin"), Some((true, "")));
        assert_eq!(parse("unpin "), Some((false, "")));
        assert_eq!(distance("provider/model", "provider/modl"), 1);
        assert_eq!(distance("モデル", "モデル"), 0);
        assert!(distance("other/model", "provider/modl") > 1);
    }
}
