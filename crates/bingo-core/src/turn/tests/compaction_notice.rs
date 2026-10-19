use super::*;

#[tokio::test]
async fn compaction_hooks_bracket_the_cut() {
    let provider = ScriptedProvider::new(vec![]);
    let mut cfg = config(provider, vec![]);
    cfg.compactor =
        CompactorSet::fixed(Some(ScriptedCompactor::new(vec![ScriptedCompactor::cut(
            "itm_none", 9_000, 100,
        )])));
    let hook = RecordingHook::new(false);
    cfg.hooks = HookSet::fixed(vec![hook.clone()]);
    let host = RecordingHost::new();
    run_turn(
        &cfg,
        TurnRun {
            hooks: Default::default(),
            turn: TurnId::from_raw("trn_1"),
            transcript: transcript(&history("hello")),
            cancel: CancellationToken::new(),
            kind: TurnKind::Compact { instructions: None },
        },
        &host,
    )
    .await;
    let summary = host
        .events()
        .into_iter()
        .find_map(|event| {
            if let Event::ItemCompleted { item } = event {
                matches!(item.body, ItemBody::Compaction { .. }).then_some(item)
            } else {
                None
            }
        })
        .expect("journalled cut");
    assert!(matches!(
        summary.body,
        ItemBody::Compaction {
            before: 9000,
            after: 100,
            window: Some(window),
            ..
        } if window > 0
    ));
    assert_eq!(hook.calls(), vec!["compact:Start", "compact:End"]);
    assert!(host.kinds().contains(&"compacted".to_string()));
}
