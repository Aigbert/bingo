use crate::{Confirmation, ID, REQUEST, Shared};
use bingo_sdk::*;
use bingo_services::schedule::{Listing, Row};
use serde_json::{Value, json};
pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        ID
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Pane","props":{"owner":ID}}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(crate::error)?;
        let listing = crate::listing(&cx.host).await?;
        let mut shared = self.0.lock().map_err(crate::error)?;
        let cursor = shared.entry(cx.session.clone()).or_default();
        cursor.seen = Some(std::time::Instant::now());
        let at = listing
            .entries
            .iter()
            .position(|r| r.id == cursor.selected)
            .unwrap_or(0);
        cursor.selected = listing
            .entries
            .get(at)
            .map(|r| r.id.clone())
            .unwrap_or_default();
        if cursor
            .confirming
            .as_ref()
            .is_some_and(|r| listing.entries.get(at) != Some(r.row()))
        {
            cursor.confirming = None;
        }
        serde_json::to_value(tree(
            &listing,
            at,
            cursor.confirming.as_ref(),
            request.viewport.columns,
            request.viewport.rows,
            jiff::Timestamp::now(),
        ))
        .map_err(crate::error)
    }
}
pub fn tree(
    listing: &Listing,
    selected: usize,
    confirming: Option<&Confirmation>,
    width: u16,
    height: u16,
    now: jiff::Timestamp,
) -> View {
    let mut children = vec![
        line(format!(" schedules · {}", holder(&listing.holder)), false),
        line(
            " ↑↓ choose  space pause/enable  r run  c confirm  d forget  → session  esc close"
                .into(),
            true,
        ),
    ];
    let (body, armed) = match confirming {
        Some(Confirmation::Confirm(row)) => crate::whole::body(row, width, height),
        _ => (entries(listing, selected, height, now), true),
    };
    children.extend(body);
    if let Some(trouble) = &listing.trouble {
        children.push(View::Badge {
            text: trouble.clone(),
            tone: bingo_sdk::Tone::Bad,
        });
    }
    children.push(line("─".repeat(usize::from(width)), true));
    children.push(line(
        footer(listing.entries.get(selected), confirming, armed),
        true,
    ));
    // A confirmation not shown whole is not offered to ⏎.
    let confirming = confirming.filter(|_| armed);
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: format!("{ID}:ui"),
                args: json!({"handler":REQUEST,"event":"press","row":listing.entries.get(selected),"confirming":confirming}),
            }),
            ..Default::default()
        }),
    }
}
/// The rows that fit, scrolled to keep the selection in view.
fn entries(listing: &Listing, selected: usize, height: u16, now: jiff::Timestamp) -> Vec<View> {
    if listing.entries.is_empty() {
        return vec![line(
            " no schedules yet · /schedule add every 2h -- check".into(),
            true,
        )];
    }
    let room = usize::from(height.saturating_sub(5)).max(1);
    let start = selected
        .saturating_sub(room / 2)
        .min(listing.entries.len().saturating_sub(room));
    listing
        .entries
        .iter()
        .enumerate()
        .skip(start)
        .take(room)
        .map(|(index, row)| entry(row, index == selected, now))
        .collect()
}
fn entry(row: &Row, selected: bool, now: jiff::Timestamp) -> View {
    line(
        format!(
            "{}{}  {}  {}  {}  {}  {}{}",
            if selected { "❯ " } else { "  " },
            row.id,
            row.spec,
            next(row, now),
            if row.enabled { "on" } else { "off" },
            row.by,
            row.text.lines().next().unwrap_or_default(),
            more(&row.text)
        ),
        !row.enabled,
    )
}
/// What a first line leaves out, said so a row never passes for the whole.
fn more(text: &str) -> String {
    match text.lines().count() {
        0 | 1 => String::new(),
        2 => "  (+1 line)".into(),
        lines => format!("  (+{} lines)", lines - 1),
    }
}
fn footer(selected: Option<&Row>, confirming: Option<&Confirmation>, armed: bool) -> String {
    if let Some(confirmation) = confirming {
        let row = confirmation.row();
        if matches!(confirmation, Confirmation::Confirm(_)) {
            return confirm_footer(row, armed);
        }
        return format!(
            " forget {} \"{} {}\"{}? its session stays.  ⏎ yes  esc",
            row.id,
            row.spec,
            row.text.lines().next().unwrap_or_default(),
            more(&row.text)
        );
    }
    selected
        .map(|row| {
            format!(
                " mode: {} · session: {}",
                row.mode.as_deref().unwrap_or("the session's"),
                row.session
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "has not fired yet".into())
            )
        })
        .unwrap_or_default()
}
fn confirm_footer(row: &Row, armed: bool) -> String {
    match armed {
        true => format!(
            " confirm {} as yours, mode {}?  ⏎ yes  esc",
            row.id,
            row.mode.as_deref().unwrap_or("the session's")
        ),
        false => format!(
            " too long to confirm here: `/schedule confirm {}` answers with all of it  esc",
            row.id
        ),
    }
}
fn next(row: &Row, now: jiff::Timestamp) -> String {
    row.next_fire
        .as_ref()
        .and_then(|at| at.parse().ok())
        .map(|at| bingo_sdk::until(now, at))
        .unwrap_or_else(|| "—".into())
}
fn holder(value: &str) -> &str {
    if value.contains("another runner") {
        "dormant — held by another bingo"
    } else if value.contains("no runner") {
        "no runner"
    } else {
        value
    }
}
pub(crate) fn line(text: String, dim: bool) -> View {
    View::Text {
        children: vec![view::TextChild::String(text)],
        style: view::TextStyle {
            dim_color: dim.then_some(true),
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}
