//! What opens a teammate's stream (`hire`'s notice): its name in its ink and
//! the role its definition gave it.
use super::Rows;
use crate::{theme, views::Drawing};
use ratatui::text::{Line, Span};

pub fn head(rows: &Rows<'_>) -> Drawing {
    let state = rows.state;
    let Some(role) = bingo_collab::role::of_state(state) else {
        return Drawing::default();
    };
    let id = &state.summary.id;
    let label = Line::from(vec![
        Span::styled(
            rows.faces.called(id),
            crate::face::style(rows.faces.ink(id)).patch(theme::bold()),
        ),
        super::room::sep(),
        Span::styled(role, theme::ash()),
    ]);
    let at = state.summary.created_at.as_millisecond();
    super::notice::said(label, at, rows.width, rows.now)
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_collab::role;

    fn child(seq: u64, event: bingo_sdk::Event) -> bingo_sdk::Frame {
        bingo_sdk::Frame {
            session: child_id(),
            ..frame(seq, event)
        }
    }

    fn screen(role: Option<&str>) -> String {
        let mut frames = vec![child(
            1,
            bingo_sdk::Event::SessionUpdated {
                summary: child_summary("reviewer"),
            },
        )];
        frames.extend(
            role.map(|text| child(2, extended(role::PLUGIN, role::KIND, role::payload(text)))),
        );
        let mut tree = spawned_tree(frames);
        tree.show(&child_id());
        let (mut ui, now) = scene();
        draw_tree(80, 12, &tree, &mut ui, now)
    }

    #[test]
    fn a_teammate_s_stream_opens_with_its_name_and_role() {
        let shown = screen(Some("reads diffs and says what breaks"));
        assert!(
            shown.contains("── reviewer · reads diffs and says what breaks ──"),
            "{shown}"
        );
        assert!(!screen(None).contains("── reviewer"));
    }
}
