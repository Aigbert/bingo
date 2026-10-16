use bingo_sdk::{
    View,
    component::{
        LedgerProps, StepState, TurnFoldProps,
        ledger::FoldGlyphs,
        row::{Ink, Piece},
    },
    render::Component,
    view::{TextChild, TextStyle},
};
use unicode_width::UnicodeWidthStr;

const FOLD: FoldGlyphs = FoldGlyphs {
    folded: "▸",
    open: "▾",
};

/// A step's row. A running step is left to the surface, whose spinner turns
/// with its clock; a still answer could not.
pub fn ledger(props: &LedgerProps, width: usize) -> View {
    if props.steps.iter().any(|s| s.state == StepState::Running) {
        return View::Engine {
            component: Component::Ledger,
            props: serde_json::to_value(props).unwrap_or_default(),
        };
    }
    let rows = props.steps.iter().map(|step| {
        let glyph = match step.state {
            StepState::Done => "✓",
            StepState::Asked => "◆",
            StepState::Running => "◌",
            StepState::Failed | StepState::Stopped => "✗",
        };
        row(step.row(glyph, width, UnicodeWidthStr::width))
    });
    View::column(rows.collect())
}

/// The folded line; a turn with nothing to fold draws nothing.
pub fn fold(props: &TurnFoldProps, width: usize) -> View {
    let rows = props.row(FOLD, width, UnicodeWidthStr::width);
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
    use bingo_sdk::{
        Upshot,
        component::{Step, TurnEnd},
    };

    fn step(verb: &str, state: StepState, upshot: Option<Upshot>) -> Step {
        Step {
            verb: verb.into(),
            target: "src/main.rs".into(),
            state,
            upshot,
            answer: None,
        }
    }

    #[test]
    fn a_done_step_draws_and_a_running_one_is_the_surfaces() {
        let done = LedgerProps {
            steps: vec![step(
                "edit",
                StepState::Done,
                Some(Upshot::Diff {
                    added: 1,
                    removed: 1,
                }),
            )],
        };
        let drawn = ledger(&done, 40).fold();
        assert!(drawn.starts_with("   ✓ edit src/main.rs"), "{drawn:?}");
        assert!(drawn.trim_end().ends_with("+1 −1"), "{drawn:?}");
        let running = LedgerProps {
            steps: vec![step("bash", StepState::Running, None)],
        };
        assert!(matches!(
            ledger(&running, 40),
            View::Engine {
                component: Component::Ledger,
                ..
            }
        ));
    }

    #[test]
    fn a_stopped_turn_folds_to_one_row() {
        let props = TurnFoldProps {
            steps: vec![
                step("read", StepState::Done, None),
                step("bash", StepState::Stopped, None),
            ],
            outcome: TurnEnd::Stopped,
            elapsed: 3_000,
            thought: false,
            is_expanded: false,
        };
        let drawn = fold(&props, 80).fold();
        let rows: Vec<&str> = drawn.lines().collect();
        assert_eq!(rows.len(), 1, "{drawn:?}");
        assert!(rows[0].starts_with("   ▸ 2 steps · read · bash · stopped"));
        assert!(rows[0].ends_with("3s"));
    }
}
