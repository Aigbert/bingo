//! `/resume ` lists the sessions it may reopen as its argument: the one in
//! view first, marked `now`, then the stored ones, each filtered by the text
//! after the command.

use bingo_sdk::component::PaletteProps;

use crate::args::Arguments;
use crate::rows::{Row, filter, session};

/// The command whose argument is a session.
const NAME: &str = "resume";

pub fn arguments(props: &PaletteProps) -> Option<Arguments> {
    let query = props.query.strip_prefix(&format!("/{NAME} "))?.trim();
    let hint = &props.commands.iter().find(|entry| entry.id == NAME)?.label;
    let current = props.session.as_str();
    let here = props
        .conversation
        .iter()
        .filter(|summary| summary.id.as_str() == current)
        .map(|summary| Row {
            now: "now".into(),
            ..session(summary, current)
        });
    let stored = props
        .sessions
        .iter()
        .map(|summary| session(summary, current));
    Some(Arguments {
        title: format!("/{NAME}  {hint}"),
        rows: here
            .chain(stored)
            .filter_map(|row| filter(row, query))
            .collect(),
        selected: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn props(query: &str) -> PaletteProps {
        serde_json::from_value(json!({
            "query": query, "open": "commands", "keyHints": {}, "bindings": {},
            "session": "ses_here", "catalogs": {},
            "action": {"name": "client", "args": {}},
            "commands": [{"id": "resume", "label": "reopen a session", "meta": {}}],
            "conversation": [{"id": "ses_here", "label": "label the total", "detail": "idle", "attention": null}],
            "sessions": [
                {"id": "ses_rooms", "label": "move rooms into chat", "detail": "3 days · 31 turns", "attention": null},
                {"id": "ses_esc", "label": "esc races every await", "detail": "5 days · 58 turns", "attention": null}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn the_session_in_view_leads_marked_now_and_the_text_filters() {
        let listed = arguments(&props("/resume ")).unwrap();
        assert_eq!(listed.title, "/resume  reopen a session");
        let rows = listed
            .rows
            .iter()
            .map(|row| (row.label.as_str(), row.now.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            [
                ("label the total", "now"),
                ("move rooms into chat", "3 days · 31 turns"),
                ("esc races every await", "5 days · 58 turns"),
            ]
        );
        let filtered = arguments(&props("/resume rooms")).unwrap();
        assert_eq!(filtered.rows.len(), 1);
        assert!(arguments(&props("/resume")).is_none(), "no argument yet");
    }
}
