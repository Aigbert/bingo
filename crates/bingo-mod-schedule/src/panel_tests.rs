use super::*;
use bingo_sdk::testing::TestHost;
use bingo_services::schedule::{Listing, Pending};
use std::sync::Arc;

struct Wake;
#[async_trait::async_trait]
impl WireService for Wake {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        match method {
            "list" => Ok(json!(Ok::<_, KernelError>(Listing::default()))),
            "pending" => {
                assert_eq!(params, json!("ses_wake"));
                Ok(json!(Ok::<_, KernelError>(Some(Pending {
                    at: "2026-10-16T12:05:00Z".into(),
                    note: "check the build".into(),
                }))))
            }
            _ => panic!("unexpected service call: {method}"),
        }
    }
}

fn context(service: bool) -> (TestHost, HookContext) {
    let host = TestHost::default();
    host.on_signal(|_, _, _, _| async { Ok(()) });
    if service {
        host.open_service(bingo_services::schedule::KEY.name(), Arc::new(Wake))
            .unwrap();
    }
    let cx = HookContext {
        session: SessionId::from_raw("ses_wake"),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    (host, cx)
}

async fn run(open: &Open, cx: &HookContext, value: Value) -> Value {
    let original = value.clone();
    open.chain(
        catalogue::COMMAND_RUN,
        cx,
        value,
        Next::new(move |value| {
            Box::pin(async move {
                assert_eq!(value, original, "the command input is untouched");
                Ok(json!(CommandOutcome::View {
                    view: View::text("original wake view")
                }))
            })
        }),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn bare_schedule_opens_and_retains_the_original_text_fallback() {
    let (_, cx) = context(true);
    let shared = Shared::default();
    let result = run(
        &Open(shared.clone()),
        &cx,
        json!({"command":"schedule","args":"","surface":"terminal"}),
    )
    .await;
    let outcome: CommandOutcome = serde_json::from_value(result).unwrap();
    let CommandOutcome::Surface { action, fallback } = outcome else {
        panic!("panel")
    };
    assert_eq!(action.name, "Pane");
    assert_eq!(action.args, json!({"plugin":ID,"id":REQUEST}));
    assert_eq!(fallback.as_deref(), Some("original wake view"));
    assert_eq!(shared.lock().unwrap()[&cx.session].selected, None);
}

#[tokio::test]
async fn schedule_arguments_text_surfaces_and_missing_services_keep_the_command() {
    for (service, input) in [
        (
            true,
            json!({"command":"schedule","args":"off","surface":"terminal"}),
        ),
        (
            true,
            json!({"command":"schedule","args":"","surface":"text"}),
        ),
        (true, json!({"command":"schedule","args":""})),
        (
            false,
            json!({"command":"schedule","args":"","surface":"terminal"}),
        ),
        (
            true,
            json!({"command":"unrelated","args":"","surface":"terminal"}),
        ),
    ] {
        let (_, cx) = context(service);
        let shared = Shared::default();
        let result = run(&Open(shared.clone()), &cx, input).await;
        assert_eq!(
            result,
            json!(CommandOutcome::View {
                view: View::text("original wake view")
            })
        );
        assert!(shared.lock().unwrap().is_empty());
    }
}

#[test]
fn the_opening_matchers_never_intercept_arguments_or_each_other() {
    let name = "schedule";
    let matcher = Open(Shared::default()).matcher();
    assert!(matcher.matches(catalogue::COMMAND_RUN, &json!({"command":name,"args":""})));
    assert!(!matcher.matches(
        catalogue::COMMAND_RUN,
        &json!({"command":name,"args":"off"})
    ));
    assert!(!matcher.matches(
        catalogue::COMMAND_RUN,
        &json!({"command":"unrelated","args":""})
    ));
}

#[tokio::test]
async fn the_schedule_panel_does_not_draw_the_separate_wake() {
    let (_, cx) = context(true);
    let shared = Shared::default();
    shared.lock().unwrap().insert(
        cx.session.clone(),
        crate::Cursor {
            selected: None,
            ..Default::default()
        },
    );
    let result = crate::draw::Drawing(shared)
        .chain(
            catalogue::UI_RENDER,
            &cx,
            json!({"component":"Pane","requestId":REQUEST,"surface":"terminal",
            "props":{"owner":ID},"viewport":{"columns":80,"rows":18}}),
            Next::new(|_| Box::pin(async { panic!("the panel draws its own view") })),
        )
        .await
        .unwrap();
    let view: View = serde_json::from_value(result).unwrap();
    assert!(view.fold().contains("Schedule"));
    assert!(!view.fold().contains("check the build"));
}
