use bingo_sdk::{
    View,
    component::{
        LedgerProps, StepState, TurnFoldProps,
        row::{Ink, Piece},
    },
    render::Component,
    view::{TextChild, TextStyle},
};
use unicode_width::UnicodeWidthStr;

/// A step's row, in the marks the surface handed over. A running step is
/// left to the surface, whose spinner turns with its clock; a still answer
/// could not.
pub fn ledger(props: &LedgerProps, width: usize) -> View {
    if props.steps.iter().any(|s| s.state == StepState::Running) {
        return View::Engine {
            component: Component::Ledger,
            props: serde_json::to_value(props).unwrap_or_default(),
        };
    }
    let rows = props.steps.iter().map(|step| {
        let glyph = props.marks.step(step.state);
        row(step.row(glyph, width, UnicodeWidthStr::width))
    });
    View::column(rows.collect())
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
    use bingo_sdk::{
        Upshot,
        component::{Step, TurnEnd, ledger::Marks},
    };

    fn marks(set: [&str; 6]) -> Marks {
        let [done, asked, running, failed, folded, open] = set.map(str::to_owned);
        Marks {
            done,
            asked,
            running,
            failed,
            folded,
            open,
        }
    }

    fn unicode() -> Marks {
        marks(["✓", "◆", "◌", "✗", "▸", "▾"])
    }

    fn step(verb: &str, state: StepState, upshot: Option<Upshot>) -> Step {
        Step {
            mark: '•',
            reads: None,
            tier: bingo_sdk::component::Tier::Change,
            verb: verb.into(),
            target: "src/main.rs".into(),
            state,
            upshot,
            answer: None,
            why: None,
            elapsed_ms: None,
            landed_at: None,
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
            marks: unicode(),
        };
        let drawn = ledger(&done, 40).fold();
        assert!(drawn.starts_with("   ✓ edit src/main.rs"), "{drawn:?}");
        assert!(drawn.trim_end().ends_with("+1 −1"), "{drawn:?}");
        let running = LedgerProps {
            steps: vec![step("bash", StepState::Running, None)],
            marks: unicode(),
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
            marks: unicode(),
        };
        let drawn = fold(&props, 80).fold();
        let rows: Vec<&str> = drawn.lines().collect();
        assert_eq!(rows.len(), 1, "{drawn:?}");
        assert!(rows[0].starts_with("   ▸ 2 steps · read · bash · stopped"));
        assert!(rows[0].ends_with("3s"));
    }

    /// The marks are the surface's: handed ASCII, the rows are ASCII.
    #[test]
    fn rows_are_drawn_in_the_marks_they_are_handed() {
        let ascii = marks(["+", "*", "o", "!", ">", "v"]);
        let steps = vec![
            step("read", StepState::Done, None),
            step("bash", StepState::Asked, None),
            step("edit", StepState::Failed, None),
        ];
        let drawn = ledger(
            &LedgerProps {
                steps: steps.clone(),
                marks: ascii.clone(),
            },
            60,
        )
        .fold();
        let marks: Vec<&str> = drawn.lines().map(|row| &row[3..4]).collect();
        assert_eq!(marks, ["+", "*", "!"], "{drawn:?}");
        let folded = TurnFoldProps {
            steps,
            outcome: TurnEnd::Done,
            elapsed: 1_000,
            thought: false,
            is_expanded: false,
            marks: ascii,
        };
        assert!(fold(&folded, 80).fold().starts_with("   > 3 steps"));
        let opened = TurnFoldProps {
            is_expanded: true,
            ..folded
        };
        assert!(fold(&opened, 80).fold().starts_with("   v 3 steps"));
    }
}
