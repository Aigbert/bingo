use super::*;
use crossterm::event::KeyEvent;

pub(super) struct Pending {
    session: SessionId,
    pub(super) focus: String,
    key: KeyEvent,
    pane: Option<(String, String)>,
}

pub(super) struct Opened {
    intent: IntentId,
    action: bingo_sdk::Action,
    handle: Result<SessionHandle, KernelError>,
}

/// The site a key reaches, as [`target`] chose it.
pub(super) struct Site {
    pub(super) session: SessionId,
    pub(super) focus: String,
    asked: bool,
    /// The focused pane's drawing, rather than one of this surface's own
    /// sites, answers the key.
    pane: bool,
}

pub(super) fn send(run: &mut Run, action: bingo_sdk::Action, key: KeyEvent) {
    let Some(Site {
        session,
        focus,
        asked,
        pane,
    }) = target(run)
    else {
        return;
    };
    let pane_id = crate::pane::placed(run.session.tree.viewed())
        .into_iter()
        .find(|pane| pane.key == focus)
        .map(|pane| (pane.plugin, pane.id));
    let handle = run.session.handles.get(&session).cloned();
    let intent = run.mint(None);
    run.site_keys.insert(
        intent.clone(),
        Pending {
            session: session.clone(),
            focus,
            key,
            pane: pane_id,
        },
    );
    if let Some(handle) = handle {
        let origin = signature(run, &session, pane, &action.name);
        submit(run, session, handle, intent, (action, origin), asked);
    } else {
        open(run, session, intent, action);
    }
}

fn submit(
    run: &mut Run,
    session: SessionId,
    handle: SessionHandle,
    intent: IntentId,
    (action, origin): (bingo_sdk::Action, Option<bingo_sdk::Origin>),
    asked: bool,
) {
    if asked {
        // The action asking this question may still hold the shared queue.
        // Interaction keys already wait for their own site's acknowledgement.
        handle.submit(intent, bingo_sdk::Input::Action { action, origin });
    } else {
        run.enqueue_action(session, handle, intent, (action, origin));
    }
}

/// A key the focused pane answers is a press on that pane's drawing; every
/// other site a key reaches is this surface's own, whatever holds focus.
fn signature(run: &Run, session: &SessionId, pane: bool, name: &str) -> Option<bingo_sdk::Origin> {
    let focus = match &run.ui.focus {
        crate::focus::Focus::Pane(focus) if pane => focus,
        _ => return Some(bingo_sdk::Origin::surface(crate::SURFACE_ID)),
    };
    let state = run.session.tree.state(session)?;
    crate::pane::placed(state)
        .into_iter()
        .find(|pane| &pane.key == focus)
        .and_then(|pane| pane.signature(name))
}

pub(super) fn target(run: &Run) -> Option<Site> {
    let surface = |session: SessionId, focus: String, asked: bool| Site {
        session,
        focus,
        asked,
        pane: false,
    };
    if crate::sessions::active(&run.ui) {
        return crate::sessions::request(&run.ui)
            .map(|id| surface(run.session.tree.view().clone(), id.into(), true));
    }
    if crate::browse::active(&run.ui) {
        return crate::browse::request(&run.ui)
            .map(|id| surface(run.session.tree.view().clone(), id.into(), false));
    }
    if crate::plugins::active(&run.ui) {
        return crate::plugins::request(&run.ui)
            .map(|id| surface(run.session.tree.view().clone(), id.into(), false));
    }
    if crate::palette::active(&run.ui) {
        return crate::palette::request(&run.ui).map(|id| {
            surface(
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
        return Some(surface(owner.summary.id.clone(), open.id.to_string(), true));
    }
    if let crate::focus::Focus::Pane(focus) = &run.ui.focus {
        return Some(Site {
            session: run.session.tree.viewed().summary.id.clone(),
            focus: focus.clone(),
            asked: false,
            pane: true,
        });
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
    let session = pending.session.clone();
    run.ui.sync_panes(run.session.tree.viewed());
    let current =
        target(run).filter(|site| site.session == pending.session && site.focus == pending.focus);
    match opened.handle {
        Ok(handle) => {
            run.session.handles.insert(session, handle.clone());
            if let Some(site) = current {
                let origin = signature(run, &site.session, site.pane, &opened.action.name);
                let action = (opened.action, origin);
                submit(run, site.session, handle, opened.intent, action, site.asked);
            } else if let Some(pending) = run.site_keys.remove(&opened.intent) {
                run.mine.remove(&opened.intent);
                super::typeahead::retry(run, pending.focus, pending.key);
            }
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
    if crate::browse::active(&run.ui)
        && crate::browse::request(&run.ui) == Some(pending.focus.as_str())
    {
        browse_answer(run, outcome);
        return;
    }
    if crate::plugins::active(&run.ui)
        && crate::plugins::request(&run.ui) == Some(pending.focus.as_str())
    {
        plugins_answer(run, outcome);
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
    pane_answer(run, &pending, outcome);
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
        crate::palette::result(
            &mut run.ui,
            &run.session.tree,
            &action,
            vouched(result, PALETTE),
            now,
        )
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
    run.typeahead.forget_site(&pending.focus);
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

fn pane_answer(run: &mut Run, pending: &Pending, outcome: &IntentOutcome) {
    let Some((plugin, id)) = &pending.pane else {
        return;
    };
    if pending.session != *run.session.tree.view() {
        return;
    }
    let IntentOutcome::Applied { result } = outcome else {
        return;
    };
    let Ok(action) = serde_json::from_value::<bingo_sdk::Action>(result["surface"].clone()) else {
        return;
    };
    let effects = crate::pane::reply::result(
        &mut run.ui,
        &run.session.tree,
        &action,
        plugin,
        id,
        vouched(result, plugin),
        run.detection.now(),
    );
    run.apply(effects);
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

/// The browse site's answer names the surface action it chose.
fn browse_answer(run: &mut Run, outcome: &IntentOutcome) {
    if let IntentOutcome::Applied { result } = outcome
        && let Ok(action) = serde_json::from_value::<bingo_sdk::Action>(result["surface"].clone())
    {
        let vouched = vouched(result, BROWSE);
        let now = run.detection.now();
        let effects = crate::browse::result(&mut run.ui, &run.session.tree, &action, vouched, now);
        run.apply(effects);
    }
}

/// The plugin panel's answer names the surface action it chose.
fn plugins_answer(run: &mut Run, outcome: &IntentOutcome) {
    if let IntentOutcome::Applied { result } = outcome
        && let Ok(action) = serde_json::from_value::<bingo_sdk::Action>(result["surface"].clone())
    {
        let vouched = vouched(result, PLUGINS);
        let now = run.detection.now();
        let effects = crate::plugins::result(&mut run.ui, &run.session.tree, &action, vouched, now);
        run.apply(effects);
    }
}

/// The built-ins that draw the plugin panel, the palette and browse. The
/// sessions directory runs no line, so it needs no word from the kernel.
const PLUGINS: &str = "plugins";
const PALETTE: &str = "palette";
const BROWSE: &str = "browse";

/// Only the kernel writes `drawnBy` into an ack, never a mod's answer: the
/// answer itself sits under `surface`. It vouches for a site only when it
/// names the built-in that owns that site.
fn vouched(result: &serde_json::Value, owner: &str) -> bool {
    result["drawnBy"] == owner
}

pub(super) fn notices(run: &mut Run) {
    for notice in crate::keys::commands::bindings(&run.ui, run.session.tree.viewed()).notices {
        run.ui.notify(Level::Warn, notice, Instant::now());
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The palette vouching for an answer is no word on the plugin panel's.
    #[test]
    fn a_site_is_vouched_for_only_by_the_built_in_that_owns_it() {
        let palette = json!({"drawnBy": PALETTE});
        assert!(vouched(&palette, PALETTE));
        assert!(!vouched(&palette, PLUGINS));
        assert!(!vouched(&palette, BROWSE));
        assert!(!vouched(&json!({"drawnBy": "rogue"}), PLUGINS));
        assert!(!vouched(&json!({}), PALETTE));
    }
}

#[cfg(test)]
mod opening_tests;
