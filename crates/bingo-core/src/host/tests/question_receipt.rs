use super::*;

struct Asks;
#[async_trait]
impl Tool for Asks {
    fn spec(&self) -> ToolSpec {
        EchoTool { read_only: true }.spec()
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        ToolTraits {
            read_only: true,
            trusted: true,
            ..Default::default()
        }
    }
    async fn call(&self, _: Value, cx: &ToolContext) -> Result<ToolOutput, ToolError> {
        cx.call
            .ask(
                InteractionKind::Form {
                    title: Some("Fixture".into()),
                    questions: vec![],
                },
                vec![AnswerSpec::Cancel],
            )
            .await
            .map_err(|error| ToolError::Failed(error.message))?;
        Ok(ToolOutput::text("continued"))
    }
}

#[tokio::test]
async fn a_question_receipt_names_the_call_that_asked() {
    let host = Host::build(
        vec![
            TestPlugin::boxed(
                &PROVIDER,
                vec![Contribution::Provider(ScriptedProvider::new(vec![
                    Script::Events(tool_call("Echo", json!({}))),
                    Script::Events(text("done")),
                ]))],
            ),
            TestPlugin::boxed(&TOOLS, vec![Contribution::Tool(Arc::new(Asks))]),
        ],
        HostConfig::new(env()).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let mut client = commands::Client::open(&host).await;
    client.handle.submit(
        IntentId::mint(),
        Input::text("ask", Origin::surface("test")),
    );
    let mut source = None;
    let mut receipts = Vec::new();
    while let Some(frame) = client.events.next().await {
        match &frame.event {
            Event::ItemStarted { item } if matches!(item.body, ItemBody::ToolCall { .. }) => {
                source = Some(item.id.clone())
            }
            Event::InteractionOpened { interaction } => {
                assert_eq!(interaction.item, source, "the ask must belong to its call");
                client.handle.answer(
                    IntentId::mint(),
                    interaction.id.clone(),
                    Answer::Cancel,
                    Activation::Keyboard,
                );
            }
            Event::ItemCompleted { item }
                if matches!(item.body, ItemBody::QuestionAnswer { .. }) =>
            {
                receipts.push(item.clone())
            }
            Event::TurnCompleted { .. } => break,
            _ => {}
        }
    }
    assert!(source.is_some());
    assert_eq!(receipts.len(), 1);
    assert!(matches!(&receipts[0].body, ItemBody::QuestionAnswer { item, .. } if item == &source));
    host.shutdown().await;
}
