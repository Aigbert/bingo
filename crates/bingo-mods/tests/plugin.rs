use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::{CatalogKind, Env, HostApi};
use serde_json::json;

#[tokio::test]
async fn an_explicit_module_is_listed_with_its_granted_runtime() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("main.ts"),
        "export function register(on) { on('tool.call', () => ({parts:[]})); }",
    )
    .unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let fixture = catalog
        .entries
        .iter()
        .find(|entry| entry.id == "fixture")
        .expect("module is discovered");
    assert_eq!(fixture.meta["runtime"], "module");
    assert_eq!(fixture.meta["tier"], "rewrite");
    assert_eq!(fixture.meta["uses"], json!(["state"]));
    assert_eq!(fixture.meta["state"]["kind"], "on");
    host.shutdown().await;
}

struct Echo;
#[async_trait::async_trait]
impl bingo_sdk::WireService for Echo {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(
            bingo_sdk::Face::new("fixture")
                .method::<serde_json::Value, serde_json::Value>("echo", false),
        )
    }
    async fn call(
        &self,
        _: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, bingo_sdk::ServiceError> {
        Ok(input)
    }
}

#[tokio::test]
async fn a_module_wraps_a_service_on_the_real_host_chain() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('fixture.echo', async ($, e, next) => ({...await next(e), wrapped:true})); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"value":1}))
        .await
        .unwrap();
    assert_eq!(result, json!({"value":1, "wrapped":true}));
    host.shutdown().await;
}

#[tokio::test]
async fn a_catalogue_hook_cannot_mint_grants() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('prompt.autocomplete', async ($, e, next) => { const result=await next(e); return {...result, entries:result.entries.map(row => row.id==='fixture' ? {...row, meta:{...row.meta, tier:'full', uses:['model']}} : row)}; }); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let fixture = catalog
        .entries
        .iter()
        .find(|entry| entry.id == "fixture")
        .unwrap();
    assert_eq!(fixture.meta["tier"], "rewrite");
    assert_eq!(fixture.meta["uses"], json!(["state"]));
    host.shutdown().await;
}

#[tokio::test]
async fn dollar_state_is_granted_and_each_operation_is_raised_once() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), r#"
      export function register(on) {
        let writes = 0;
        on('fixture.echo', async ($, e) => {
          await $.state.set('key', e);
          return {...await $.state.get('key'), writes};
        });
        on('state.set', async ($, e, next) => { writes++; return next({...e, value:{...e.value, seen:true}}); });
      }
    "#).unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"value":1}))
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({"version":1, "value":{"value":1,"seen":true}, "writes":1})
    );
    host.shutdown().await;
}

#[tokio::test]
async fn an_undeclared_state_call_is_refused_in_words() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('fixture.echo', async ($) => { try { await $.state.get('key'); return {error:'not refused'}; } catch(error) { return {error:String(error)}; } }); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({}))
        .await
        .unwrap();
    assert!(
        result["error"].as_str().unwrap().contains("state grant"),
        "{result}"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn a_timed_out_inner_hook_gets_a_fresh_catch_budget() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("main.ts"),
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e, next) => ({...await next(e), outer:true}));
        on('fixture.echo', {test:1}, () => { while(true) {} }).catch(($, e, next) => ({
          kind:next.error.kind, called:next.called, budget:next.budget.ms
        }));
      }
    "#,
    )
    .unwrap();
    let config =
        HostConfig::new(Env::rooted(home.path())).with_layer("test", json!({"mods":{"hookMs":50}}));
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        config,
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"test":1}))
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({"kind":"timeout", "called":false, "budget":1000, "outer":true})
    );
    host.shutdown().await;
}

mod support;

#[tokio::test]
async fn dollar_reads_the_session_fold_and_freezes_its_plugin_identity() {
    let fixture = support::Fixture::new(
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e) => {
          let frozen = false;
          try { $.plugin.name = 'foreign'; } catch { frozen = true; }
          return { id:await $.session.id({session:e.session}),
            cwd:await $.session.cwd({session:e.session}),
            config:await $.config.list({session:e.session}), plugin:$.plugin, frozen };
        });
      }
    "#,
        &["session", "config"],
        json!({}),
    )
    .await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    assert_eq!(answer["id"], json!(fixture.attachment.session));
    assert_eq!(answer["cwd"], json!(fixture.home.path()));
    let current = fixture
        .host
        .open(
            bingo_sdk::SessionSelector::ById {
                id: fixture.attachment.session.clone(),
            },
            bingo_sdk::ClientIdentity {
                name: "check".into(),
                surface: "test".into(),
            },
            bingo_sdk::OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(answer["config"], json!(current.snapshot.config));
    assert_eq!(
        answer["plugin"],
        json!({"name":"fixture","root":fixture.root,"version":"1"})
    );
    assert_eq!(answer["frozen"], true);
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn an_append_keeps_its_calling_hook_across_the_mailbox() {
    use bingo_sdk::*;
    use futures::StreamExt;
    let mut fixture = support::Fixture::new(
        r#"
      export function register(on) {
        let calls = 0;
        on('session.append', {kind:'probe'}, async ($, e, next) => {
          calls++;
          if (calls === 1) await $.session.append({key:'probe',value:1});
          return next(e);
        });
        on('fixture.echo', () => ({calls}));
      }
    "#,
        &["session"],
        json!({}),
    )
    .await;
    fixture
        .host
        .extend(&fixture.attachment.session, "test", "probe", json!(0))
        .await
        .unwrap();
    let mut appended = 0;
    while appended < 2 {
        let frame = fixture.attachment.events.next().await.unwrap();
        if matches!(frame.event, Event::Extension { ref kind, .. } if kind == "probe") {
            appended += 1;
        }
    }
    let answer = fixture.call("fixture", "echo", json!({})).await;
    assert_eq!(
        answer["calls"], 1,
        "the calling hook re-entered across the actor mailbox"
    );
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn a_registered_tool_is_prefixed_and_offered_by_the_catalogue() {
    let fixture = support::Fixture::new(
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e) => ({name:await $.tool.register({
          name:'hello',description:'Say hello',inputSchema:{type:'object'}
        }, async ($, input) => ({parts:[{type:'text',text:'hello'}],isError:false}))}));
      }
    "#,
        &["tool"],
        json!({}),
    )
    .await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    assert_eq!(answer["name"], "plugin__fixture__hello");
    let tools = fixture
        .host
        .catalog(bingo_sdk::CatalogKind::Tools)
        .await
        .unwrap();
    assert!(
        tools
            .entries
            .iter()
            .any(|tool| tool.id == "plugin__fixture__hello")
    );
    fixture.host.shutdown().await;
}

#[cfg(test)]
mod registered {
    use super::*;
    use bingo_sdk::*;
    use futures::StreamExt;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Model(AtomicUsize);
    static MANIFEST: PluginManifest = PluginManifest {
        id: "fixture.model",
        version: "1",
        uses: &[],
        provides: &[],
        requires: &[],
        config: None,
    };
    #[async_trait::async_trait]
    impl Plugin for Model {
        fn manifest(&self) -> &'static PluginManifest {
            &MANIFEST
        }
        fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
            registrar.provider(Arc::new(Model(AtomicUsize::new(0))));
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl Provider for Model {
        fn id(&self) -> &str {
            "fixture.model"
        }
        fn endpoint(&self, _: &str) -> EndpointCapabilities {
            EndpointCapabilities::default()
        }
        async fn stream(
            &self,
            request: ModelRequest,
            _: CancellationToken,
        ) -> Result<ModelStream, ProviderError> {
            assert!(
                request
                    .tools
                    .iter()
                    .any(|tool| tool.name == "plugin__fixture__hello")
            );
            let mut events = Vec::new();
            let reason = if self.0.fetch_add(1, Ordering::SeqCst).is_multiple_of(2) {
                events.push(Ok(ModelEvent::ToolCall {
                    id: "call".into(),
                    name: "plugin__fixture__hello".into(),
                    input: "{}".into(),
                }));
                UnifiedFinish::ToolCalls
            } else {
                events.extend([
                    Ok(ModelEvent::TextStart { id: "a".into() }),
                    Ok(ModelEvent::TextDelta {
                        id: "a".into(),
                        delta: "done".into(),
                    }),
                    Ok(ModelEvent::TextEnd { id: "a".into() }),
                ]);
                UnifiedFinish::Stop
            };
            events.push(Ok(ModelEvent::Finish {
                usage: Usage::default(),
                finish_reason: FinishReason::unified(reason),
            }));
            Ok(Box::pin(futures::stream::iter(events)))
        }
    }
    const SOURCE: &str = r#"
      export function register(on) {
        on('session.start', async ($) => {
          await $.tool.register({name:'hello',description:'fixture',inputSchema:{type:'object'}},
            async ($) => { await $.session.append({key:'ran',value:'first'}); return {parts:[{type:'text',text:'first'}],isError:false}; });
          await $.command.register({name:'hello',hint:'fixture',args:{kind:'none'},family:'fixture'},
            async ($) => ({kind:'applied',message:'first'}));
          await $.session.append({key:'ready',value:true});
        });
      }
    "#;
    async fn extension(session: &mut Attachment, key: &str) -> serde_json::Value {
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("expected extension")
                    .unwrap();
            if let Event::InteractionOpened { ref interaction } = frame.event {
                assert!(matches!(
                    interaction.kind,
                    InteractionKind::Permission { .. }
                ));
                session.handle.answer(
                    IntentId::mint(),
                    interaction.id.clone(),
                    Answer::AllowOnce,
                    Activation::Programmatic,
                );
            }
            assert!(
                !matches!(frame.event, Event::TurnCompleted { .. }),
                "turn ended before {key}"
            );
            if let Event::Extension { kind, payload, .. } = frame.event
                && kind == key
            {
                return payload;
            }
        }
    }
    #[tokio::test]
    async fn registered_callbacks_run_and_reload_before_the_next_call() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("fixture");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("plugin.json"), json!({"name":"fixture","version":"1","module":"main.ts","uses":["tool","command","session"]}).to_string()).unwrap();
        std::fs::write(root.join("main.ts"), SOURCE).unwrap();
        let host = Host::build(
            vec![
                Box::new(ModsPlugin::default().with_dirs(vec![root.clone()])),
                Box::new(Model(AtomicUsize::new(0))),
            ],
            HostConfig::new(Env::rooted(home.path())).with_layer(
                "offline",
                json!({"provider":"fixture.model","model":"offline"}),
            ),
        )
        .await
        .unwrap();
        let mut session = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: home.path().into(),
                        ..Default::default()
                    },
                },
                ClientIdentity {
                    name: "fixture".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap();
        extension(&mut session, "ready").await;
        session.handle.submit(
            IntentId::mint(),
            Input::text("run", Origin::surface("test")),
        );
        assert_eq!(extension(&mut session, "ran").await, "first");
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("turn completion")
                    .unwrap();
            if matches!(frame.event, Event::TurnCompleted { .. }) {
                break;
            }
        }
        std::fs::write(root.join("main.ts"), SOURCE.replace("first", "second")).unwrap();
        session.handle.submit(
            IntentId::mint(),
            Input::text("run again", Origin::surface("test")),
        );
        assert_eq!(extension(&mut session, "ran").await, "second");
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("turn completion")
                    .unwrap();
            if matches!(frame.event, Event::TurnCompleted { .. }) {
                break;
            }
        }
        let intent = IntentId::mint();
        session.handle.submit(
            intent.clone(),
            Input::Action {
                action: Action {
                    name: "hello".into(),
                    args: serde_json::Value::Null,
                },
                origin: Some(Origin::surface("tui")),
            },
        );
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("command ack")
                    .unwrap();
            if let Event::IntentAck {
                intent: ack,
                outcome: verdict,
            } = frame.event
                && ack == intent
            {
                assert!(
                    serde_json::to_value(verdict)
                        .unwrap()
                        .to_string()
                        .contains("second")
                );
                break;
            }
        }
        host.shutdown().await;
    }
}

#[tokio::test]
async fn state_pins_its_owner_and_store_keys_and_delete_share_the_shelf() {
    let fixture = support::Fixture::new(r#"
      export function register(on) {
        on('state.set', async ($, e, next) => next({...e,plugin:'foreign'}));
        on('fixture.echo', async ($, e) => {
          let foreign = '';
          try { await $['bingo.plugin.state'].set({plugin:'foreign',scope:e.session,key:'x',value:1}); }
          catch (error) { foreign = error.message; }
          const state = await $.state.set('x', 2);
          await $.store.set('a', 3);
          let conflict = '';
          try { await $.store.set('a', 4, 0); } catch(error) {conflict = error.message;}
          const keys = await $.store.keys();
          const removed = await $.store.delete('a');
          return {state,foreign,conflict,keys,removed,empty:await $.store.keys()};
        });
      }
    "#, &["state","store"], json!({})).await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    assert_eq!(answer["state"], json!({"version":1,"value":2}));
    assert!(
        answer["foreign"]
            .as_str()
            .unwrap()
            .contains("another plugin")
    );
    assert!(
        answer["conflict"]
            .as_str()
            .unwrap()
            .contains("held version: 1")
    );
    assert_eq!(answer["keys"], json!(["a"]));
    assert_eq!(answer["removed"], true);
    assert_eq!(answer["empty"], json!([]));
    fixture.host.shutdown().await;
}

#[cfg(test)]
mod model_nouns {
    use super::*;
    use bingo_sdk::*;
    use futures::StreamExt;
    use std::sync::Arc;
    struct Model;
    static MANIFEST: PluginManifest = PluginManifest {
        id: "fixture.side",
        version: "1",
        uses: &[],
        provides: &[],
        requires: &[],
        config: None,
    };
    #[async_trait::async_trait]
    impl Plugin for Model {
        fn manifest(&self) -> &'static PluginManifest {
            &MANIFEST
        }
        fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
            registrar.provider(Arc::new(Model));
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl Provider for Model {
        fn id(&self) -> &str {
            "fixture.side"
        }
        fn endpoint(&self, _: &str) -> EndpointCapabilities {
            EndpointCapabilities::default()
        }
        async fn stream(
            &self,
            request: ModelRequest,
            _: CancellationToken,
        ) -> Result<ModelStream, ProviderError> {
            let usage = if request.session.is_none() {
                Usage {
                    input_tokens: 7,
                    output_tokens: 2,
                    ..Default::default()
                }
            } else {
                Usage::default()
            };
            Ok(Box::pin(futures::stream::iter([
                Ok(ModelEvent::TextStart { id: "a".into() }),
                Ok(ModelEvent::TextDelta {
                    id: "a".into(),
                    delta: "yes".into(),
                }),
                Ok(ModelEvent::TextEnd { id: "a".into() }),
                Ok(ModelEvent::Finish {
                    usage,
                    finish_reason: FinishReason::unified(UnifiedFinish::Stop),
                }),
            ])))
        }
    }
    async fn fixture() -> (tempfile::TempDir, Arc<Host>, Attachment) {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("fixture");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("plugin.json"), json!({"name":"fixture","version":"1","module":"main.ts","uses":["model","clock","session"]}).to_string()).unwrap();
        std::fs::write(root.join("main.ts"), r#"
          export function register(on) {
            async function side($) {
              if (await $.model.classify({text:'a question',labels:['yes','no']}) !== 'yes') throw new Error('wrong label');
              await $.model.fork({prompt:'side question'});
            }
            on('session.start', async ($) => {
              await $.clock.after(0, async ($) => {await side($); await $.session.append({key:'timer-done',value:true});});
            });
            on('turn.step', async ($, e, next) => {await side($); return next(e);});
          }
        "#).unwrap();
        let host = Host::build(
            vec![
                Box::new(ModsPlugin::default().with_dirs(vec![root])),
                Box::new(Model),
            ],
            HostConfig::new(Env::rooted(home.path())).with_layer(
                "offline",
                json!({"provider":"fixture.side","model":"offline"}),
            ),
        )
        .await
        .unwrap();
        let session = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: home.path().into(),
                        ..Default::default()
                    },
                },
                ClientIdentity {
                    name: "fixture".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap();
        (home, host, session)
    }
    #[tokio::test]
    async fn classify_and_fork_bill_the_turn_but_timer_calls_bill_the_session() {
        let (_home, host, mut session) = fixture().await;
        let mut state = session.snapshot.clone();
        let mut charges = Vec::new();
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("timer callback did not finish")
                    .unwrap();
            state.apply(&frame);
            if let Event::UsageCharged {
                turn,
                plugin,
                usage,
            } = &frame.event
            {
                assert_eq!(plugin, "fixture");
                assert_eq!(usage.input_tokens, 7);
                charges.push(turn.clone());
            }
            if matches!(frame.event, Event::Extension {ref kind,..} if kind == "timer-done") {
                break;
            }
        }
        assert_eq!(charges, vec![None, None]);
        session.handle.submit(
            IntentId::mint(),
            Input::text("main", Origin::surface("test")),
        );
        loop {
            let frame =
                tokio::time::timeout(std::time::Duration::from_secs(15), session.events.next())
                    .await
                    .expect("main turn did not finish")
                    .unwrap();
            state.apply(&frame);
            if let Event::UsageCharged { turn, .. } = &frame.event {
                charges.push(turn.clone());
            }
            if matches!(frame.event, Event::TurnCompleted { .. }) {
                break;
            }
        }
        assert_eq!(charges.len(), 4);
        assert!(charges[2].is_some());
        assert_eq!(charges[2], charges[3]);
        assert_eq!(state.last_turn.unwrap().usage.input_tokens, 14);
        assert_eq!(state.summary.usage.input_tokens, 28);
        host.shutdown().await;
    }
}

#[tokio::test]
async fn model_helpers_without_a_model_grant_are_refused_in_words() {
    let fixture = support::Fixture::new(
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e) => {
          const errors = [];
          for (const method of ['classify','fork']) {
            try {await $.model[method]({session:e.session,text:'x',labels:['yes'],prompt:'x'});}
            catch(error) {errors.push(error.message);}
          }
          return {errors};
        });
      }
    "#,
        &[],
        json!({}),
    )
    .await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    let errors = answer["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|error| error.as_str().unwrap().contains("model grant"))
    );
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn surfaces_fold_duplicate_attachments_and_each_detach() {
    use bingo_sdk::*;
    let fixture = support::Fixture::new("export function register(on) { on('fixture.echo', ($, e) => $.session.surfaces({session:e.session})); }", &["session"], json!({})).await;
    let identity = ClientIdentity {
        name: "duplicate".into(),
        surface: "test".into(),
    };
    let first = fixture
        .host
        .open(
            SessionSelector::ById {
                id: fixture.attachment.session.clone(),
            },
            identity.clone(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let second = fixture
        .host
        .open(
            SessionSelector::ById {
                id: fixture.attachment.session.clone(),
            },
            identity.clone(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    async fn count(fixture: &support::Fixture, expected: u64) {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let state = fixture
                    .host
                    .session_state(&fixture.attachment.session)
                    .await
                    .unwrap();
                let held = state
                    .signals
                    .get("bingo.mods")
                    .into_iter()
                    .flat_map(|values| values.values())
                    .find(|value| value["client"]["name"] == "duplicate")
                    .and_then(|value| value["count"].as_u64())
                    .unwrap_or(0);
                if held == expected {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    count(&fixture, 2).await;
    drop(first);
    count(&fixture, 1).await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    assert_eq!(
        answer
            .as_array()
            .unwrap()
            .iter()
            .filter(|value| value["name"] == "duplicate")
            .count(),
        1
    );
    drop(second);
    count(&fixture, 0).await;
    let answer = fixture
        .call(
            "fixture",
            "echo",
            json!({"session":fixture.attachment.session}),
        )
        .await;
    assert!(
        answer
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value["name"] != "duplicate")
    );
    fixture.host.shutdown().await;
}
