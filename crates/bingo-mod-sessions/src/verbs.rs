use bingo_sdk::*;
pub(crate) fn label(summary: &SessionSummary, now: jiff::Timestamp) -> String {
    summary
        .title
        .clone()
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            summary
                .history
                .first_ask
                .clone()
                .filter(|text| !text.trim().is_empty())
        })
        .unwrap_or_else(|| format!("untitled · {}", ago(now, summary.updated_at)))
}
