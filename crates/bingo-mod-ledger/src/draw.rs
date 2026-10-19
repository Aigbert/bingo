use bingo_sdk::{View, component::LedgerProps};

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
            teammate: None,
        };
        for state in [StepState::Done, StepState::Asked, StepState::Running] {
            step.state = state;
            let props = LedgerProps {
                steps: vec![step.clone()],
                marks: Marks { asked: "?".into() },
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
