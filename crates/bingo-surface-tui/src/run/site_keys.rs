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
