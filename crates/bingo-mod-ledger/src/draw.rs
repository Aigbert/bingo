use bingo_sdk::{
    View,
    component::{
        LedgerProps, TurnFoldProps,
        row::{Ink, Piece},
    },
    view::{TextChild, TextStyle},
};
use unicode_width::UnicodeWidthStr;

/// Each step declares its own spinner, breath and landing on the surface clock.
pub fn ledger(props: &LedgerProps, width: usize) -> View {
    View::column(
        props
            .steps
            .iter()
            .map(|step| step.drawing(&props.marks, width))
            .collect(),
    )
}

/// The folded line; a turn with nothing to fold draws nothing.
pub fn fold(props: &TurnFoldProps, width: usize) -> View {
    let rows = props.row(width, UnicodeWidthStr::width);
    View::column(rows.into_iter().map(row).collect())
}

fn row(pieces: Vec<Piece>) -> View {
    let children = pieces.into_iter().map(piece).collect();
    text(children, None, false)
}

fn piece(piece: Piece) -> TextChild {
    let (color, dim) = match piece.ink {
        Ink::Plain => (None, false),
        Ink::Dim => (None, true),
        Ink::Ash => (Some("ash"), false),
        Ink::Good => (Some("good"), false),
        Ink::Bad => (Some("bad"), false),
        Ink::Glow => (Some("glow"), false),
        Ink::Presence => (Some("presence"), false),
    };
    TextChild::Node(Box::new(text(
        vec![TextChild::String(piece.text)],
        color,
        dim,
    )))
}

fn text(children: Vec<TextChild>, color: Option<&str>, dim: bool) -> View {
    View::Text {
        children,
        style: TextStyle {
            color: color.map(str::to_owned),
            dim_color: Some(dim),
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::component::{Step, StepState, Tier, ledger::Marks};
    #[test]
    fn rows_are_drawn_in_the_marks_they_are_handed() {
        let mut step = Step {
            mark: '@',
            verb: "inspect".into(),
            target: "target".into(),
            reads: None,
            tier: Tier::Change,
            state: StepState::Done,
            upshot: None,
            answer: None,
            elapsed_ms: None,
            why: None,
            landed_at: None,
        };
        for state in [StepState::Done, StepState::Asked, StepState::Running] {
            step.state = state;
            let props = LedgerProps {
                steps: vec![step.clone()],
                marks: Marks {
                    asked: "?".into(),
                    done: "✓".into(),
                    running: "◌".into(),
                    failed: "✗".into(),
                    folded: "▸".into(),
                    open: "▾".into(),
                },
            };
            let drawn = ledger(&props, 60);
            assert!(drawn.fold().starts_with(if state == StepState::Asked {
                "   ? inspect"
            } else {
                "   @ inspect"
            }));
            if state == StepState::Running {
                assert!(serde_json::to_string(&drawn).unwrap().contains("spin"));
            }
        }
    }
}
