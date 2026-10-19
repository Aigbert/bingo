use crate::{ID, REQUEST, Shared, rows};
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
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if request.props["owner"] != ID {
            return next.run(value).await;
        }
        let listing = crate::listing(&cx.host).await?;
        let rows = listing.entries.clone();
        let mut shared = self.0.lock().map_err(crate::error)?;
        let cursor = shared.entry(cx.session.clone()).or_default();
        cursor.seen = Some(std::time::Instant::now());
        let at = rows::selected(&rows, cursor.selected.as_deref());
        cursor.selected = rows.get(at).map(|row| row.id.clone());
        serde_json::to_value(tree(
            &listing,
            cursor.selected.as_deref(),
            &jiff::tz::TimeZone::system(),
            request.viewport.columns,
            request.viewport.rows,
            jiff::Timestamp::now(),
            request.viewport.glyphs,
        ))
        .map_err(crate::error)
    }
}
pub fn tree(
    listing: &Listing,
    selected: Option<&str>,
    zone: &jiff::tz::TimeZone,
    width: u16,
    height: u16,
    now: jiff::Timestamp,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let rows = listing
        .entries
        .iter()
        .map(|row| entry(row, Some(row.id.as_str()) == selected, now, zone, glyphs))
        .collect();
    let mut view = Sheet {
        head: "Schedule".into(),
        fact: Some(pick::text(
            format!(
                "{} running",
                listing.entries.iter().filter(|r| r.enabled).count()
            ),
            "dim",
        )),
        top: vec![],
        rows,
        selected: selected.map(str::to_owned),
        foot: vec![pick::line([
            pick::text("/schedule every 30m ", "code"),
            pick::text("<prompt> adds one", "dim"),
        ])],
        pick: true,
    }
    .view(width, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(component::SiteHints::new([
            component::KeyPair::new("↑↓", "schedule"),
            component::KeyPair::new("space", "pause"),
            component::KeyPair::new("⏎", "run now"),
            component::KeyPair::new("esc", "close"),
        ]));
        layout.on_key = Some(Action {
            name: format!("{ID}:ui"),
            args: json!({"handler":REQUEST,"event":"press","row":listing.entries.iter().find(|row| Some(row.id.as_str()) == selected),"rows":listing.entries}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
fn entry(
    row: &Row,
    selected: bool,
    now: jiff::Timestamp,
    zone: &jiff::tz::TimeZone,
    glyphs: glyph::GlyphSet,
) -> pick::Row {
    use bingo_sdk::pick::{bold, line, text};
    let mut lamp = text(
        glyph::Glyph::Lamp.spell(glyphs),
        if row.enabled { "glow" } else { "faint" },
    );
    if row.enabled
        && let View::Text { style, .. } = &mut lamp
    {
        style.motion = Some(Box::new(view::TextMotion::Breathe {
            with: "you".into(),
            period_ms: 2400,
            depth: 0.5,
        }));
    }
    let ink = if row.enabled { "muted" } else { "dim" };
    let prompt = if selected {
        bold(bingo_services::schedule::visible(&row.text), "fg")
    } else {
        text(bingo_services::schedule::visible(&row.text), ink)
    };
    let next = if row.enabled {
        row.next_fire
            .as_ref()
            .and_then(|at| at.parse().ok())
            .map(|at| format!("next {}", next_time(now, at, zone)))
            .unwrap_or_default()
    } else {
        "paused".into()
    };
    pick::Row::new(
        &row.id,
        line([
            lamp,
            text("  ", ink),
            pick::pad(text(&row.spec, ink), 16),
            prompt,
        ]),
    )
    .right(text(next, "dim"))
}
fn next_time(now: jiff::Timestamp, at: jiff::Timestamp, zone: &jiff::tz::TimeZone) -> String {
    let now_local = now.to_zoned(zone.clone());
    let at_local = at.to_zoned(zone.clone());
    if now_local.date().checked_add(jiff::Span::new().days(1)).ok() == Some(at_local.date()) {
        format!("tomorrow {:02}:{:02}", at_local.hour(), at_local.minute())
    } else {
        bingo_sdk::until(now, at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_next_day_fire_names_its_local_hour() {
        let now = "2026-10-19T14:00:00Z".parse().unwrap();
        let at = "2026-10-20T09:00:00Z".parse().unwrap();
        assert_eq!(
            next_time(now, at, &jiff::tz::TimeZone::UTC),
            "tomorrow 09:00"
        );
    }
    #[test]
    fn schedules_use_the_shared_window_and_declared_lamp_clock() {
        let listing = Listing {
            entries: (0..12)
                .map(|i| Row {
                    id: i.to_string(),
                    spec: "every 30m".into(),
                    next_fire: None,
                    enabled: true,
                    by: "you".into(),
                    mode: None,
                    text: format!("prompt {i}"),
                    session: None,
                })
                .collect(),
            ..Default::default()
        };
        let view = tree(
            &listing,
            Some("11"),
            &jiff::tz::TimeZone::UTC,
            100,
            18,
            jiff::Timestamp::UNIX_EPOCH,
            Default::default(),
        );
        assert!(view.fold().contains("8 of 12"));
        assert!(!view.fold().contains("prompt 0"));
        assert!(serde_json::to_string(&view).unwrap().contains("2400"));
        bingo_sdk::testing::assert_separate_hints(&view);
    }
}
