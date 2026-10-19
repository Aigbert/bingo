use super::*;

pub(super) fn refocus(run: &mut Run) {
    refocus_at(run, run.now().wall);
}

pub(super) fn refocus_at(run: &mut Run, now: jiff::Timestamp) {
    let open_ids: std::collections::HashSet<_> = run
        .session
        .tree
        .sessions()
        .flat_map(|state| state.interactions.iter().map(|open| open.id.clone()))
        .collect();
    run.received_asks.retain(|id, _| open_ids.contains(id));
    for id in open_ids {
        run.received_asks.entry(id).or_insert(now);
    }

    let open = run.session.tree.open_interaction().map(|(_, open)| open);
    let changed = run.ui.dialog.current() != open.map(|open| &open.id);
    run.ui.dialog.focus_on(open);
    if changed && let Some(open) = open {
        run.ui.dialog.aside = !matches!(open.kind, bingo_sdk::InteractionKind::Login { .. })
            && (crate::sessions::active(&run.ui)
                || crate::palette::active(&run.ui)
                || crate::plugins::active(&run.ui)
                || crate::ask::aside::recent(
                    run.received_asks.get(&open.id).copied().unwrap_or(now),
                    run.ui.last_typed,
                ));
    }
}

pub(super) fn stop_interaction(
    run: &mut Run,
    session: SessionId,
    interaction: bingo_sdk::InteractionId,
    answer: Option<bingo_sdk::Answer>,
) {
    run.discard_actions(std::slice::from_ref(&session));
    let answer = answer.map(|answer| (run.mint(crate::run::Submission::Ordinary(None)), answer));
    let interrupt = run.mint(crate::run::Submission::Ordinary(None));
    let stop = move |handle: &SessionHandle| {
        if let Some((intent, answer)) = answer {
            handle.answer(intent, interaction, answer, bingo_sdk::Activation::Keyboard);
        }
        handle.interrupt(interrupt, InterruptScope::Head);
    };
    if let Some(handle) = run.session.handles.get(&session) {
        stop(handle);
        return;
    }
    let host = run.host.clone();
    run.spawn(async move {
        let attachment = host
            .open(
                SessionSelector::ById {
                    id: session.clone(),
                },
                identity(),
                OpenOptions::default(),
            )
            .await?;
        stop(&attachment.handle);
        Ok(Reply::Handle(session, attachment.handle))
    });
}

/// An answer goes through the root: its handle knows which session asked.
pub(super) fn answer(
    run: &mut Run,
    interaction: bingo_sdk::InteractionId,
    answer: bingo_sdk::Answer,
    activation: bingo_sdk::Activation,
) {
    let Some(handle) = run.session.root() else {
        return run.not_yet();
    };
    let intent = run.mint(crate::run::Submission::Ordinary(None));
    handle.answer(intent, interaction, answer, activation);
}
