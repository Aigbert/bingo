//! Ask scenes start the turn that asks, open its ask when the turn reaches
//! it, and let the rest of the turn go on only once a key has answered it.
use super::*;
use bingo_sdk::{
    Answer, AnswerSpec, Caution, Interaction, InteractionId, InteractionKind, Item, ItemId,
    Preview, Question, QuestionOption, ResolvedBy, Stakes, TurnId,
};

/// The ask a scene holds open, and the turn's events that wait for it.
pub(super) struct Asking {
    interaction: Interaction,
    held: Vec<(i64, Event)>,
}

pub(super) fn is_ask(name: &str) -> bool {
    ["ask-", "caution", "question", "small-ask", "ascii-ask"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// The history the ask's turn follows, the turn, and the ask opened on the
/// step that asks.
pub(super) async fn open(capture: &Capture) -> Stage {
    let mut scene = setup::new(capture);
    scene.screen.size = Some((capture.size().width, capture.size().height));
    let caution = scene.name.starts_with("caution");
    let state = history(scene.run.session.tree.root(), caution);
    scene.run.session = Attached::new(state.clone(), SessionHandle(scene.session.clone()));
    let kernel = kernel::Kernel::new(scene.host.clone(), state);
    world::script(&kernel, &world::Switched::default());
    let env = bingo_sdk::Env::rooted(scene._home.path());
    kernel.register(&bingo_mod_ask::Ask, &env);
    let turn = scene.turns[0].clone();
    let mut stage = Stage {
        scene,
        kernel,
        calls: 0,
        provider: vec![],
        ask: None,
    };
    let t0 = turn["t0"].as_i64().unwrap() - CLOCK_ORIGIN_MS;
    stage.scene.set_time(t0.min(-OPENING_MS));
    super::super::catalogues::fetch_all(&mut stage.scene.run).await;
    let mode = if stage.scene.name.starts_with("question") {
        "acceptEdits"
    } else {
        "default"
    };
    stage.kernel.broadcast(config_in(mode));
    stage.settle().await;
    let n = if caution { 2 } else { 1 };
    start(&mut stage, &turn, n);
    stage.at(0).await;
    stage
}

/// The live chat's first turn, or the finished chat for a force push.
fn history(root: &SessionState, done: bool) -> SessionState {
    let mut state = if done {
        story::scene("done").0
    } else {
        let mut state = root.clone();
        story::fold_turn(&mut state, 0, &story::turns("live")[0]);
        state
    };
    let mut summary = state.summary.clone();
    summary.model = Some("opus-5.5".into());
    summary.provider = Some("anthropic".into());
    summary.effort = Some(bingo_sdk::Effort::High);
    summary.title = Some("steer 吞掉排队消息".into());
    state.apply(&fixture::frame(
        state.seq.0 + 1,
        Event::SessionUpdated { summary },
    ));
    state
}

fn config_in(mode: &str) -> Event {
    let mut event = fixture::permission_view(mode);
    if let Event::ConfigChanged { config } = &mut event {
        config.kernel["thinking"] = json!("high");
    }
    event
}

/// The prompt lands at the turn's start; the provider streams the steps up
/// to the ask, and everything after it waits for the answer.
fn start(stage: &mut Stage, turn: &Value, n: usize) {
    let id = format!("ember-{n}");
    let t0 = turn["t0"].as_i64().unwrap();
    let input = Input::text(turn["prompt"].as_str().unwrap(), crate::origin());
    let at = jiff::Timestamp::from_millisecond(t0).unwrap();
    for event in answers::sent(&id, IntentId::from_raw("ask-prompt"), &input, at) {
        stage.kernel.broadcast(event);
    }
    let ask = &turn["asks"][0];
    let asked = ask["at"].as_i64().unwrap();
    let item = ItemId::from_raw(format!("{id}-{}", ask["item"]));
    let mut held = Vec::new();
    for (at, event) in story::provider_events(n, turn) {
        let event = unasked(event, &item);
        let waits = at > asked || finishes(&event, &item);
        let list = if waits {
            &mut held
        } else {
            &mut stage.provider
        };
        let promoted = promoted(&event);
        list.push((at - CLOCK_ORIGIN_MS, event));
        if let Some(id) = promoted {
            list.push((at - CLOCK_ORIGIN_MS, timeline::promotion(id)));
        }
    }
    let interaction = interaction(stage, turn, &id, &item);
    stage
        .provider
        .push((asked - CLOCK_ORIGIN_MS, opened(stage, &item, asked)));
    stage.provider.push((
        asked - CLOCK_ORIGIN_MS,
        Event::InteractionOpened {
            interaction: interaction.clone(),
        },
    ));
    stage.ask = Some(Asking { interaction, held });
}

/// The asked step has not begun: its own start is the answer.
fn unasked(mut event: Event, asked: &ItemId) -> Event {
    if let Event::ItemStarted { item } = &mut event
        && &item.id == asked
        && let ItemBody::ToolCall { began_at, .. } = &mut item.body
    {
        *began_at = None;
    }
    event
}

/// Whether the event finishes the asked step, which only an answer can do.
fn finishes(event: &Event, asked: &ItemId) -> bool {
    matches!(event, Event::ItemCompleted { item } if &item.id == asked)
}

fn promoted(event: &Event) -> Option<ItemId> {
    match event {
        Event::ItemStarted { item } if matches!(&item.body, ItemBody::ToolCall { name, .. } if name == "bash") => {
            Some(item.id.clone())
        }
        _ => None,
    }
}

/// The step as the kernel marks it when it asks: still waiting to begin.
fn opened(stage: &Stage, item: &ItemId, asked: i64) -> Event {
    let mut item = asked_item(stage, item);
    if let ItemBody::ToolCall { asked_at, .. } = &mut item.body {
        *asked_at = Some(jiff::Timestamp::from_millisecond(asked).unwrap());
    }
    Event::ItemUpdated { item }
}

fn asked_item(stage: &Stage, id: &ItemId) -> Item {
    stage
        .provider
        .iter()
        .find_map(|(_, event)| match event {
            Event::ItemStarted { item } if &item.id == id => Some(item.clone()),
            _ => None,
        })
        .unwrap()
}

fn interaction(stage: &Stage, turn: &Value, id: &str, item: &ItemId) -> Interaction {
    let ask = &turn["asks"][0];
    let step = &turn["items"][ask["item"].as_u64().unwrap() as usize];
    let asked = ask["at"].as_i64().unwrap();
    let opened_at = jiff::Timestamp::from_millisecond(asked).unwrap();
    let (kind, answers) = match ask["kind"].as_str().unwrap() {
        "edit" => edit(stage, step),
        "bash" => command(step, ask["why"].as_str(), None),
        "caution" => command(step, None, Some(ask)),
        "question" => form(ask),
        kind => panic!("unported ask {kind}"),
    };
    Interaction {
        id: InteractionId::from_raw("ask-1"),
        session: stage.kernel.root().summary.id.clone(),
        turn: Some(TurnId::from_raw(id)),
        item: Some(item.clone()),
        opened_at,
        guard_until: Some(opened_at + jiff::SignedDuration::from_millis(400)),
        expires_at: None,
        kind,
        answers,
    }
}

fn edit(stage: &Stage, step: &Value) -> (InteractionKind, Vec<AnswerSpec>) {
    let rows = step["diff"].as_array().unwrap();
    let first = rows[0]["n"].as_u64().unwrap();
    let old = rows.iter().filter(|row| row["s"] != "+").count();
    let new = rows.iter().filter(|row| row["s"] != "-").count();
    let mut unified = format!("@@ -{first},{old} +{first},{new} @@\n");
    for row in rows {
        let sign = row["s"].as_str().unwrap();
        unified.push_str(&format!("{sign}{}\n", row["t"].as_str().unwrap()));
    }
    let target = step["arg"].as_str().unwrap();
    let cwd = stage.kernel.root().summary.cwd.clone();
    let directory = &target[..=target.rfind('/').unwrap()];
    let kind = InteractionKind::Permission {
        tool: "edit".into(),
        input: json!({"target":target}),
        summary: format!("Edit {target}"),
        preview: Some(Box::new(Preview::Diff { unified })),
        session_scope: Some(format!("Edit({cwd}/{directory})")),
        caution: None,
    };
    let answers = vec![
        AnswerSpec::AllowOnce,
        AnswerSpec::AllowSession,
        AnswerSpec::Deny,
    ];
    (kind, answers)
}

fn command(
    step: &Value,
    why: Option<&str>,
    caution: Option<&Value>,
) -> (InteractionKind, Vec<AnswerSpec>) {
    let target = step["arg"].as_str().unwrap();
    let home = crate::paths::home().unwrap_or("/home/me");
    let stakes = caution.map(|ask| {
        let lost: Vec<String> = ask["lost"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap().to_owned())
            .collect();
        Stakes {
            line: format!("drops {} commits that only origin/dev has", lost.len()),
            items: lost,
        }
    });
    let kind = InteractionKind::Permission {
        tool: "bash".into(),
        input: json!({"target":target,"command":target}),
        summary: target.into(),
        preview: Some(Box::new(Preview::Command {
            command: target.into(),
            cwd: format!("{home}/bingo-improve"),
            description: why.map(str::to_owned),
            stakes,
        })),
        session_scope: caution.is_none().then(|| "Bash(cargo test:*)".to_owned()),
        caution: caution.map(|ask| Caution {
            reason: ask["caution"].as_str().unwrap().into(),
            act: Some("force-push".into()),
        }),
    };
    let mut answers = vec![AnswerSpec::AllowOnce];
    if caution.is_none() {
        answers.push(AnswerSpec::AllowSession);
    }
    answers.extend([AnswerSpec::Deny, AnswerSpec::AllowChanged]);
    (kind, answers)
}

/// What the model's ask tool opens: a form of its questions.
fn form(ask: &Value) -> (InteractionKind, Vec<AnswerSpec>) {
    let questions = ask["qs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| Question {
            question: q["question"].as_str().unwrap().into(),
            header: q["header"].as_str().map(str::to_owned),
            options: q["options"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, option)| QuestionOption {
                    id: i.to_string(),
                    label: option["label"].as_str().unwrap().into(),
                    description: option["description"].as_str().map(str::to_owned),
                    role: None,
                    preview: option["preview"].as_str().map(str::to_owned),
                })
                .collect(),
            free_text: true,
            multi: q["multi"] == true,
        })
        .collect();
    let kind = InteractionKind::Form {
        title: None,
        questions,
    };
    (kind, vec![AnswerSpec::Form, AnswerSpec::Cancel])
}

impl Stage {
    /// An answer the surface sent closes the ask; one that lets the step run
    /// starts it now, and the rest of the turn goes on from here.
    pub(super) fn resolve(&mut self) -> bool {
        let Some(asking) = &self.ask else {
            return false;
        };
        let id = asking.interaction.id.clone();
        let Some(answer) = self
            .scene
            .session
            .answers()
            .into_iter()
            .find(|(asked, ..)| asked == &id)
            .map(|(_, answer, _)| answer)
        else {
            return false;
        };
        let asking = self.ask.take().unwrap();
        self.kernel.broadcast(Event::InteractionResolved {
            id,
            answer: answer.clone(),
            by: ResolvedBy::Client {
                name: "bingo".into(),
                surface: crate::SURFACE_ID.into(),
            },
        });
        if !matches!(answer, Answer::Deny { .. } | Answer::Cancel) {
            self.go_on(asking);
        }
        true
    }

    fn go_on(&mut self, asking: Asking) {
        let now = self.scene.elapsed();
        let wall = self.scene.run.now().wall;
        let asked = asking.interaction.item.clone();
        for (at, mut event) in asking.held {
            if let Event::ItemCompleted { item } = &mut event
                && Some(&item.id) == asked.as_ref()
            {
                begun(item, asking.interaction.opened_at, wall);
            }
            self.provider.push((at.max(now), event));
        }
    }
}

/// The asked step began when it was answered, and finished no earlier.
fn begun(item: &mut Item, asked: jiff::Timestamp, now: jiff::Timestamp) {
    if let ItemBody::ToolCall {
        asked_at, began_at, ..
    } = &mut item.body
    {
        *asked_at = Some(asked);
        *began_at = Some(now);
    }
    item.completed_at = item.completed_at.map(|done| done.max(now));
}
