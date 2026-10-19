//! A step that started or addressed a teammate, as the crew draws it: the
//! teammate's name as the verb and its colour for the mark, and whether it has
//! reported back. Every other step passes on to the ledger's owner.
use bingo_sdk::component::{LedgerProps, Step, StepState, Teammate};
use bingo_sdk::*;
use serde_json::{Value, json};

/// Outside the built-in ledger, so a teammate's row is answered here first.
#[derive(Clone, Copy)]
pub struct Rows;

#[async_trait::async_trait]
impl Interceptor for Rows {
    fn id(&self) -> &str {
        "crew.ledger"
    }
    fn matcher(&self) -> LinkMatcher {
        let component = render::Component::Ledger;
        LinkMatcher {
            prefilter: json!({ "component": component }),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let Some(request) = voiced(&value) else {
            return next.run(value).await;
        };
        let width = usize::from(request.0.viewport.columns);
        serde_json::to_value(ledger(&request.1, width)).map_err(crate::error)
    }
}

/// The request and its props, when any step in it has a teammate.
fn voiced(value: &Value) -> Option<(render::RenderRequest, LedgerProps)> {
    let request: render::RenderRequest = serde_json::from_value(value.clone()).ok()?;
    let props: LedgerProps = serde_json::from_value(request.props.clone()).ok()?;
    props
        .steps
        .iter()
        .any(|step| step.teammate.is_some())
        .then_some((request, props))
}

/// Each step on its own row; only a teammate's is drawn in its voice.
pub fn ledger(props: &LedgerProps, width: usize) -> View {
    View::column(
        props
            .steps
            .iter()
            .map(|step| match &step.teammate {
                Some(teammate) => row(step, teammate, props, width),
                None => step.drawing(&props.marks, width),
            })
            .collect(),
    )
}

fn row(step: &Step, teammate: &Teammate, props: &LedgerProps, width: usize) -> View {
    let mut step = step.clone();
    step.verb = teammate.name.clone();
    if !matches!(step.state, StepState::Failed | StepState::Running) {
        step.upshot = Some(Upshot::plain(word(teammate)));
        step.landed_at = teammate.reported_at.or(step.landed_at);
    }
    step.drawing_in(&props.marks, width, &teammate.ink)
}

/// A task handed out reads `started` until its teammate posts back.
fn word(teammate: &Teammate) -> &'static str {
    match teammate.reported_at {
        Some(_) => "reported",
        None => "started",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::component::{Tier, ledger::Marks};

    fn step(teammate: Option<Teammate>) -> Step {
        Step {
            mark: '@',
            verb: "spawn".into(),
            target: "find every caller of drain()".into(),
            reads: Some(Reads::Text),
            tier: Tier::Change,
            state: StepState::Done,
            upshot: Some(Upshot::plain("{\"name\":\"scout\"}")),
            answer: None,
            elapsed_ms: None,
            why: None,
            landed_at: Some(900),
            teammate,
        }
    }

    fn scout(reported_at: Option<i64>) -> Option<Teammate> {
        Some(Teammate {
            name: "scout".into(),
            ink: "crew1".into(),
            reported_at,
        })
    }

    fn props(step: Step) -> LedgerProps {
        LedgerProps {
            steps: vec![step],
            marks: Marks {
                asked: "◆".into()
            },
        }
    }

    #[test]
    fn a_teammate_row_names_it_and_says_whether_it_reported() {
        let started = ledger(&props(step(scout(None))), 80).fold();
        assert!(started.starts_with("   @ scout   find every caller of drain()"));
        assert!(started.ends_with("started"), "{started}");
        let reported = ledger(&props(step(scout(Some(1600)))), 80);
        assert!(reported.fold().ends_with("reported"));
        let wire = serde_json::to_value(&reported).unwrap();
        let mark = &wire["children"][0]["children"][1];
        assert_eq!(mark["color"], "crew1");
        assert_eq!(mark["motion"]["atMs"], 1600, "it lands when it reports");
    }

    #[test]
    fn a_running_teammate_spins_in_its_ink() {
        let mut running = step(scout(None));
        running.state = StepState::Running;
        let wire = serde_json::to_value(ledger(&props(running), 80)).unwrap();
        let mark = &wire["children"][0]["children"][1];
        assert_eq!(mark["color"], "crew1");
        assert_eq!(mark["motion"]["kind"], "spin");
    }

    #[test]
    fn any_other_step_is_left_to_the_ledger() {
        let plain = json!({"component":"Ledger","requestId":"itm_1","surface":"terminal",
            "props": props(step(None)),"viewport":{"columns":60,"rows":1}});
        assert!(voiced(&plain).is_none());
        let named = json!({"component":"Ledger","requestId":"itm_1","surface":"terminal",
            "props": props(step(scout(None))),"viewport":{"columns":60,"rows":1}});
        assert!(voiced(&named).is_some());
    }
}
