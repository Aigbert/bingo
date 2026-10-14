use super::*;
use crossterm::event::KeyEvent;

pub(super) struct Pending {
    session: SessionId,
    focus: String,
    key: KeyEvent,
}

pub(super) fn send(run: &mut Run, action: bingo_sdk::Action, key: KeyEvent) {
    let focus = if let Some((_, open)) = run
        .session
        .tree
        .open_interaction()
        .filter(|_| !run.ui.dialog.aside)
    {
        open.id.to_string()
    } else if let crate::focus::Focus::Pane(focus) = &run.ui.focus {
        focus.clone()
    } else {
        return;
    };
    let Some(handle) = run.session.writer() else {
        return run.not_yet();
    };
    let intent = run.mint(None);
    run.site_keys.insert(
        intent.clone(),
        Pending {
            session: run.session.tree.viewed().summary.id.clone(),
            focus,
            key,
        },
    );
    run.ui.actions.push(
        run.session.tree.viewed().summary.id.clone(),
        handle,
        intent,
        action,
    );
}

pub(super) fn answered(run: &mut Run, intent: &IntentId, outcome: &IntentOutcome) {
    let Some(pending) = run.site_keys.remove(intent) else {
        return;
    };
    if let Some((_, open)) = run.session.tree.open_interaction()
        && open.id.as_str() == pending.focus
        && matches!(
            open.kind,
            bingo_sdk::InteractionKind::Question(_) | bingo_sdk::InteractionKind::Form { .. }
        )
        && let IntentOutcome::Applied { result } = outcome
        && let Ok(answer) = serde_json::from_value::<bingo_sdk::Answer>(result["answer"].clone())
        && open.answers.contains(&answer.spec())
    {
        run.ui.dialog.answered = true;
        run.site_queue.retain(|(site, _)| site != &pending.focus);
        run.answer(open.id.clone(), answer, bingo_sdk::Activation::Keyboard);
        return;
    }
    if let Some((_, open)) = run.session.tree.open_interaction()
        && open.id.as_str() == pending.focus
        && !run.ui.dialog.aside
        && matches!(outcome, IntentOutcome::Applied { result } if result["handled"] == false)
    {
        let effects = input::interaction_unhandled(
            &mut run.ui,
            &run.session.tree,
            pending.key,
            run.detection.now(),
        );
        run.apply(effects);
        return;
    }
    if matches!(outcome, IntentOutcome::Applied { result } if result["handled"] == false)
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

pub(super) fn notices(run: &mut Run) {
    for notice in crate::keys::commands::bindings(&run.ui, run.session.tree.viewed()).notices {
        run.ui.notify(Level::Warn, notice, Instant::now());
    }
}

fn focus(run: &Run) -> Option<String> {
    if let Some((_, open)) = run.session.tree.open_interaction() {
        return Some(open.id.to_string());
    }
    if let crate::focus::Focus::Pane(focus) = &run.ui.focus {
        return Some(focus.clone());
    }
    None
}
pub(super) fn blocked(run: &Run) -> bool {
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
        if run.site_keys.values().any(|p| p.focus == site) {
            break;
        }
        if !run.ui.dialog.aside
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
