use super::*;
use crossterm::event::KeyEvent;

pub(super) struct Pending {
    session: SessionId,
    focus: String,
    key: KeyEvent,
}

pub(super) struct Opened {
    intent: IntentId,
    action: bingo_sdk::Action,
    handle: Result<SessionHandle, KernelError>,
}

pub(super) fn send(run: &mut Run, action: bingo_sdk::Action, key: KeyEvent) {
    let Some((session, focus, asked)) = target(run) else {
        return;
    };
    let handle = run.session.handles.get(&session).cloned();
    if !asked && handle.is_none() {
        return run.not_yet();
    }
    let intent = run.mint(None);
    run.site_keys.insert(
        intent.clone(),
        Pending {
            session: session.clone(),
            focus,
            key,
        },
    );
    if asked {
        // The action asking this question may still hold the shared queue.
        // Interaction keys already wait for their own site's acknowledgement.
        if let Some(handle) = handle {
            handle.submit(intent, Input::Action { action });
        } else {
            open(run, session, intent, action);
        }
    } else if let Some(handle) = handle {
        run.enqueue_action(session, handle, intent, action);
    }
}

fn target(run: &Run) -> Option<(SessionId, String, bool)> {
    if crate::sessions::active(&run.ui) {
        return crate::sessions::request(&run.ui)
            .map(|id| (run.session.tree.view().clone(), id.into(), true));
    }
    if crate::palette::active(&run.ui) {
        return crate::palette::request(&run.ui).map(|id| {
            (
                run.session.tree.viewed().summary.id.clone(),
                id.into(),
                false,
            )
        });
    }
    if let Some((owner, open)) = run
        .session
        .tree
        .open_interaction()
        .filter(|_| !run.ui.dialog.aside)
    {
        return Some((owner.summary.id.clone(), open.id.to_string(), true));
    }
    if let crate::focus::Focus::Pane(focus) = &run.ui.focus {
        return Some((
            run.session.tree.viewed().summary.id.clone(),
            focus.clone(),
            false,
        ));
    }
    None
}

fn open(run: &mut Run, session: SessionId, intent: IntentId, action: bingo_sdk::Action) {
    let host = run.host.clone();
    run.spawn(async move {
        let handle = host
            .open(
                SessionSelector::ById { id: session },
                identity(),
                OpenOptions::default(),
            )
            .await
            .map(|attachment| attachment.handle);
        Ok(Reply::SiteKeyOpened(Box::new(Opened {
            intent,
            action,
            handle,
        })))
    });
}

pub(super) fn opened(run: &mut Run, opened: Opened) {
    let Some(pending) = run.site_keys.get(&opened.intent) else {
        run.mine.remove(&opened.intent);
        return;
    };
    let live = run
        .session
        .tree
        .state(&pending.session)
        .is_some_and(|owner| {
            owner
                .interactions
                .iter()
                .any(|open| open.id.as_str() == pending.focus)
        });
    if !live {
        run.site_keys.remove(&opened.intent);
        run.mine.remove(&opened.intent);
        drain(run);
        return;
    }
    match opened.handle {
        Ok(handle) => {
            run.session
                .handles
                .insert(pending.session.clone(), handle.clone());
            handle.submit(
                opened.intent,
                Input::Action {
                    action: opened.action,
                },
            );
        }
        Err(error) => run.ack(&opened.intent, &IntentOutcome::Rejected { error }),
    }
}

pub(super) fn answered(run: &mut Run, intent: &IntentId, outcome: &IntentOutcome) {
    let Some(pending) = run.site_keys.remove(intent) else {
        return;
    };
    if crate::sessions::active(&run.ui)
        && crate::sessions::request(&run.ui) == Some(pending.focus.as_str())
    {
        directory_answer(run, &pending, outcome);
        return;
    }
    if crate::palette::active(&run.ui)
        && crate::palette::request(&run.ui) == Some(pending.focus.as_str())
    {
        palette_answer(run, &pending, outcome);
        return;
    }
    if interaction_answer(run, &pending, outcome) || interaction_unhandled(run, &pending, outcome) {
        return;
    }
    pane_unhandled(run, &pending, outcome);
}

/// The palette's site either picked an action or handed the key back to
/// the palette's own editing.
fn palette_answer(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) {
    let IntentOutcome::Applied { result } = outcome else {
        return;
    };
    let now = run.detection.now();
    let effects = if let Ok(action) =
        serde_json::from_value::<bingo_sdk::Action>(result["surface"].clone())
    {
        crate::palette::result(&mut run.ui, &run.session.tree, &action, now)
    } else if result["handled"] == false {
        input::palette_edit(&mut run.ui, &run.session.tree, pending.key, now)
    } else {
        Vec::new()
    };
    run.apply(effects);
}

/// A key the open question's site turned into one of the offered answers.
fn interaction_answer(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) -> bool {
    let Some((_, open)) = run.session.tree.open_interaction() else {
        return false;
    };
    if open.id.as_str() != pending.focus
        || !matches!(
            open.kind,
            bingo_sdk::InteractionKind::Question(_) | bingo_sdk::InteractionKind::Form { .. }
        )
    {
        return false;
    }
    let IntentOutcome::Applied { result } = outcome else {
        return false;
    };
    let Ok(answer) = serde_json::from_value::<bingo_sdk::Answer>(result["answer"].clone()) else {
        return false;
    };
    if !open.answers.contains(&answer.spec()) {
        return false;
    }
    let interaction = open.id.clone();
    run.ui.dialog.answered = true;
    run.site_queue.retain(|(site, _)| site != &pending.focus);
    super::interactions::answer(run, interaction, answer, bingo_sdk::Activation::Keyboard);
    true
}

/// A key the open interaction's site left alone goes to the dialog itself.
fn interaction_unhandled(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) -> bool {
    let focused = run
        .session
        .tree
        .open_interaction()
        .is_some_and(|(_, open)| open.id.as_str() == pending.focus);
    if !focused || run.ui.dialog.aside || !unhandled(outcome) {
        return false;
    }
    let effects = input::interaction_unhandled(
        &mut run.ui,
        &run.session.tree,
        pending.key,
        run.detection.now(),
    );
    run.apply(effects);
    true
}

/// A key a pane's site left alone goes to the pane's own keys, while that
/// pane is still the one in focus.
fn pane_unhandled(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) {
    if unhandled(outcome)
        && pending.session == run.session.tree.viewed().summary.id
        && matches!(&run.ui.focus, crate::focus::Focus::Pane(key) if key == &pending.focus)
    {
        let effects = input::pane_keys(
            &mut run.ui,
            &run.session.tree,
            pending.key,
            run.detection.now(),
        );
        run.apply(effects);
    }
}

fn unhandled(outcome: &IntentOutcome) -> bool {
    matches!(outcome, IntentOutcome::Applied { result } if result["handled"] == false)
}

pub(super) fn notices(run: &mut Run) {
    for notice in crate::keys::commands::bindings(&run.ui, run.session.tree.viewed()).notices {
        run.ui.notify(Level::Warn, notice, Instant::now());
    }
}

fn focus(run: &Run) -> Option<String> {
    // An unhandled printable key sets the ask aside; finish its already queued
    // burst in the composer instead of dropping the remaining letters.
    target(run).map(|(_, focus, _)| focus).or_else(|| {
        run.session
            .tree
            .open_interaction()
            .map(|(_, open)| open.id.to_string())
    })
}
fn palette_waiting(run: &Run) -> bool {
    crate::palette::waiting(&run.ui, &run.session.tree, run.detection.now())
}
pub(super) fn blocked(run: &Run) -> bool {
    if palette_waiting(run)
        || crate::sessions::waiting(&run.ui, &run.session.tree, run.detection.now())
    {
        return true;
    }
    focus(run).is_some_and(|focus| {
        run.site_keys.values().any(|p| p.focus == focus)
            || run.site_queue.iter().any(|(site, _)| site == &focus)
    })
}
pub(super) fn queue(run: &mut Run, key: KeyEvent) {
    if let Some(focus) = focus(run) {
        run.site_queue.push_back((focus, key));
    }
}
pub(super) fn drain(run: &mut Run) {
    while let Some((site, key)) = run.site_queue.front().cloned() {
        if focus(run).as_ref() != Some(&site) {
            run.site_queue.pop_front();
            continue;
        }
        if palette_waiting(run)
            || crate::sessions::waiting(&run.ui, &run.session.tree, run.detection.now())
        {
            break;
        }
        if run.site_keys.values().any(|p| p.focus == site) {
            break;
        }
        if crate::focus::owner(&run.ui, &run.session.tree) == crate::focus::Owner::Interaction
            && run
                .session
                .tree
                .open_interaction()
                .is_some_and(|(owner, open)| crate::ask::waiting(&run.ui, owner, open))
        {
            break;
        }
        run.site_queue.pop_front();
        let effects = input::on_key(&mut run.ui, &run.session.tree, key, run.detection.now());
        run.apply(effects);
    }
}

fn directory_answer(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) {
    if let IntentOutcome::Applied { result } = outcome {
        let now = run.detection.now();
        let effects = if let Ok(action) =
            serde_json::from_value::<bingo_sdk::Action>(result["surface"].clone())
        {
            crate::sessions::result(&mut run.ui, &action, now)
        } else if result["handled"] == false {
            input::palette_edit(&mut run.ui, &run.session.tree, pending.key, now)
        } else {
            Vec::new()
        };
        run.apply(effects);
    }
}
