//! Catalogue work is owned by the actor and returns through its mailbox.
use super::*;

pub(crate) struct Completed {
    turn: TurnId,
    status: TurnStatus,
    usage: Usage,
    items: Vec<Item>,
    notices: Vec<ItemBody>,
}

pub(super) fn complete(actor: &Actor, turn: TurnId, status: TurnStatus, items: Vec<Item>) {
    let usage = actor
        .state
        .turn
        .as_ref()
        .map(|turn| turn.usage)
        .unwrap_or_default();
    let answer = items
        .iter()
        .filter(|item| item.turn.as_ref() == Some(&turn))
        .filter_map(|item| match &item.body {
            ItemBody::Assistant { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let input = bingo_sdk::catalogue::Turn {
        turn: turn.clone(),
        outcome: status.clone(),
        usage,
        answer,
    };
    let hooks = actor.config.hooks.clone();
    let mut cx = actor.hook_context();
    cx.turn = Some(turn.clone());
    let mailbox = actor.mailbox.clone();
    let cancel = actor
        .running
        .as_ref()
        .map(|run| run.cancel.clone())
        .unwrap_or_else(|| actor.cancel.child_token());
    actor.tracker.spawn(async move {
        let report = crate::chain::Report::default();
        let text = super::running::supervised(
            async {
                crate::chain::complete::text(input, &hooks.links().await, &cx, report.clone()).await
            },
            &cancel,
        )
        .await;
        let mut notices = report.take();
        if let Some(Ok(Some(text))) = text
            && !text.is_empty()
        {
            notices.push(ItemBody::Notice {
                level: Level::Info,
                code: "TURN_COMPLETE".into(),
                text,
            });
        }
        mailbox.send(Msg::CompletionReady(Box::new(Completed {
            turn,
            status,
            usage,
            items,
            notices,
        })));
    });
}

pub(super) async fn finished(actor: &mut Actor, completed: Completed) -> Result<Flow, KernelError> {
    let Completed {
        turn,
        status,
        usage,
        items,
        notices,
    } = completed;
    if !actor.is_running(&turn) {
        return Ok(Flow::Continue);
    }
    if let Some(running) = actor.running.take() {
        running.finish();
    }
    actor
        .publish(
            Event::TurnCompleted {
                turn: turn.clone(),
                status,
                usage,
            },
            None,
        )
        .await?;
    for notice in notices {
        actor.record(notice).await?;
    }
    actor.after_turn(turn, items);
    if let Some(reason) = actor.closing.take() {
        return actor.finish_close(reason).await;
    }
    actor.drain_queue().await?;
    Ok(Flow::Continue)
}

pub(super) fn rewrite(actor: &Actor, event: Event) {
    let hooks = actor.config.hooks.clone();
    let cx = actor.hook_context();
    let mailbox = actor.mailbox.clone();
    let work = crate::chain::observe::Work::Task(Box::pin(async move {
        let links = hooks.links().await;
        let report = crate::chain::Report::default();
        let changed = match event {
            Event::Extension {
                plugin,
                kind,
                payload,
            } => crate::chain::append::rows(
                bingo_sdk::catalogue::Append {
                    plugin,
                    kind,
                    payload,
                },
                &links,
                &cx,
                report.clone(),
            )
            .await
            .map(|input| Event::Extension {
                plugin: input.plugin,
                kind: input.kind,
                payload: input.payload,
            }),
            Event::ConfigChanged { config } => {
                crate::chain::config::describe(config, &links, &cx, report.clone())
                    .await
                    .map(|config| Event::ConfigChanged { config })
            }
            event => Ok(event),
        };
        for notice in report.take() {
            let _ = mailbox.record(notice).await;
        }
        if let Ok(event) = changed {
            mailbox.send(Msg::CatalogueEvent(Box::new(event)));
        }
    }));
    if let Some(queue) = &actor.observed {
        let _ = queue.send(work);
    }
}
