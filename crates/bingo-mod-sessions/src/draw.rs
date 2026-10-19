use crate::{
    Shared,
    rows::{Row, Target},
    state::Cursor,
};
use bingo_sdk::{component::SessionsProps, *};
use serde_json::{Value, json};
pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "sessions"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Sessions"}),
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
        let props: SessionsProps =
            serde_json::from_value(request.props.clone()).map_err(crate::error)?;
        let (view, wanted) = {
            let mut state = self.0.lock().map_err(crate::error)?;
            let cursor = state
                .cursors
                .entry((props.session.clone(), request.request_id.clone()))
                .or_default();
            self.drawn(cursor, &props, &request)
        };
        let owner = (props.session.clone(), request.request_id.clone());
        crate::probe::start(self.0.clone(), cx.host.clone(), owner, wanted, props.now);
        serde_json::to_value(view).map_err(crate::error)
    }
}
impl Drawing {
    /// The directory as it stands, and what its visible rows still need.
    fn drawn(
        &self,
        cursor: &mut Cursor,
        props: &SessionsProps,
        request: &render::RenderRequest,
    ) -> (View, crate::probe::Wanted) {
        cursor.sync(&props.query);
        let rows = crate::rows::listed(props, cursor);
        cursor.settle(&rows);
        let sheet = sheet(props, &rows, cursor, request.viewport.glyphs);
        let visible = sheet
            .window(request.viewport.rows)
            .map(|i| &rows[i])
            .collect::<Vec<_>>();
        let wanted = crate::probe::wanted(&mut cursor.probed, &visible, props);
        let view = finish(
            sheet.view(
                request.viewport.columns,
                request.viewport.rows,
                request.viewport.glyphs,
            ),
            props,
            &request.request_id,
        );
        (view, wanted)
    }
}
#[cfg(test)]
pub(crate) fn tree(
    props: &SessionsProps,
    rows: &[Row],
    cursor: &Cursor,
    request: &str,
    height: u16,
    width: u16,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    finish(
        sheet(props, rows, cursor, glyphs).view(width, height, glyphs),
        props,
        request,
    )
}
fn finish(mut view: View, props: &SessionsProps, request: &str) -> View {
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(component::SiteHints::new([
            component::KeyPair::new("↑↓", "session"),
            component::KeyPair::new("⏎", "open"),
            component::KeyPair::new("esc", "close"),
        ]));
        layout.on_key = Some(Action {
            name: "sessions:ui".into(),
            args: json!({"handler":request,"event":"press","props":props}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
fn sheet(
    props: &SessionsProps,
    rows: &[Row],
    cursor: &Cursor,
    glyphs: glyph::GlyphSet,
) -> bingo_sdk::sheet::Sheet {
    use bingo_sdk::{pick, sheet::Sheet};
    let list = rows
        .iter()
        .map(|row| row_view(row, props, cursor, glyphs))
        .collect();
    let fact = if props.query.is_empty() {
        "needs you first".into()
    } else {
        format!("{} of {}", rows.len(), props.sessions.len())
    };
    let query = if props.query.is_empty() {
        pick::line([
            pick::text(glyph::Glyph::Caret.spell(glyphs), "glow"),
            pick::text("type to filter by title or directory", "dim"),
        ])
    } else {
        pick::line([
            pick::text(&props.query, "fg"),
            pick::text(glyph::Glyph::Caret.spell(glyphs), "glow"),
        ])
    };
    Sheet {
        head: "Sessions".into(),
        fact: Some(pick::text(fact, "dim")),
        top: vec![
            pick::line([pick::text("filter  ", "dim"), query]),
            pick::text("", "dim"),
        ],
        rows: list,
        selected: cursor.selected.as_ref().map(identity),
        foot: vec![],
        pick: true,
    }
}
pub(crate) fn identity(target: &Target) -> String {
    match target {
        Target::Session(id) => id.to_string(),
        Target::Group(id) => id.clone(),
        Target::More => "more".into(),
    }
}
fn row_view(
    row: &Row,
    props: &SessionsProps,
    cursor: &Cursor,
    glyphs: glyph::GlyphSet,
) -> pick::Row {
    use bingo_sdk::pick::{bold, line, text};
    let summary = props
        .sessions
        .iter()
        .find(|s| row.target == Target::Session(s.id.clone()));
    let asking = summary.is_some_and(|s| s.asking);
    let working = summary.is_some_and(|s| s.busy);
    let selected = cursor.selected.as_ref() == Some(&row.target);
    let mut lamp = text(
        glyph::Glyph::Lamp.spell(glyphs),
        if asking {
            "amberGlow"
        } else if working {
            "glow"
        } else {
            "faint"
        },
    );
    if (asking || working)
        && let View::Text { style, .. } = &mut lamp
    {
        style.motion = Some(Box::new(view::TextMotion::Breathe {
            with: "you".into(),
            period_ms: if asking { 1100 } else { 1800 },
            depth: if asking { 0.6 } else { 0.55 },
        }));
    }
    let title = if selected {
        bold(&row.title, "fg")
    } else {
        text(&row.title, "muted")
    };
    let left = line([
        lamp,
        text("  ", "fg"),
        pick::pad(title, 26),
        text(summary.map(|s| s.cwd.as_str()).unwrap_or_default(), "dim"),
    ]);
    let right = if asking {
        text("needs you", "amber")
    } else if working {
        text("working", "muted")
    } else {
        text(
            summary
                .map(|s| when(props.now, s.updated_at))
                .unwrap_or_default(),
            "dim",
        )
    };
    pick::Row::new(identity(&row.target), left).right(right)
}

fn when(now: jiff::Timestamp, at: jiff::Timestamp) -> String {
    let seconds = (now.as_second() - at.as_second()).max(0);
    if seconds < 60 {
        "now".into()
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h", seconds / 3600)
    } else if seconds < 172800 {
        "yesterday".into()
    } else {
        at.strftime("%b %-d").to_string()
    }
}

#[cfg(test)]
mod sheet_tests {
    use super::*;
    #[test]
    fn sessions_use_the_shared_window_after_need_ordering() {
        let mut props = crate::tests::props();
        props.sessions = (0..12)
            .map(|i| crate::tests::summary(&i.to_string(), Some(&format!("session {i}"))))
            .collect();
        props.sessions[11].asking = true;
        let cursor = Cursor {
            selected: Some(Target::Session(SessionId::from_raw("11"))),
            ..Default::default()
        };
        let rows = crate::rows::listed(&props, &cursor);
        assert_eq!(rows[0].target, cursor.selected.clone().unwrap());
        let view = tree(&props, &rows, &cursor, "list", 19, 100, Default::default());
        assert!(view.fold().contains("8 of 12"));
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
        bingo_sdk::testing::assert_separate_hints(&view);
    }
}
