use super::*;

struct ScriptedAgent;

#[async_trait]
impl Agent for ScriptedAgent {
    fn id(&self) -> &str { "fake" }
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> { Ok(vec![]) }
    async fn run(&self, turn: AgentTurn) -> Result<AgentStream, ProviderError> {
        assert_eq!(turn.input, vec![ContentPart::text("hello")]);
        assert!(turn.tools.is_empty());
        let call = ExternalCall { id: "edit-1".into(), kind: CallKind::Edit,
            title: "Edit note.md".into(), locations: vec![], raw: json!({}) };
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(AgentEvent::Call(call.clone())),
            Ok(AgentEvent::Done { call, output: ToolOutput::text("written") }),
            Ok(AgentEvent::Said(ModelEvent::Finish { usage: Usage::default(),
                finish_reason: FinishReason::unified(UnifiedFinish::Stop) })),
        ])))
    }
}

#[tokio::test]
async fn an_agent_journals_one_external_call() {
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    cfg.model.as_mut().unwrap().endpoint = super::super::Endpoint::Agent(Arc::new(ScriptedAgent));
    let host = RecordingHost::new();
    let out = run(&cfg, &host, CancellationToken::new()).await;
    assert_eq!(out.status, TurnStatus::Completed);
    let completed: Vec<_> = host.events().into_iter().filter_map(|e| match e {
        Event::ItemCompleted { item } if matches!(item.body, ItemBody::ToolCall { .. }) => Some(item),
        _ => None,
    }).collect();
    assert_eq!(completed.len(), 1);
    assert!(completed[0].external());
    assert!(matches!(&completed[0].body, ItemBody::ToolCall { name, output: Some(_), .. } if name == "fake:edit"));
}
