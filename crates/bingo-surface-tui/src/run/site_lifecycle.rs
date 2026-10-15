use super::*;
fn request(ui: &Ui) -> Option<&str> {
    crate::sessions::request(ui).or_else(|| crate::palette::request(ui))
}

/// Closing consumes Ctrl+C at idle; a running turn still gets its interrupt.
pub(super) fn ctrl_c(run: &mut Run, key: crossterm::event::KeyEvent) -> bool {
    if !crate::palette::active(&run.ui) && !crate::sessions::active(&run.ui) {
        return false;
    }
    let close = run
        .ui
        .shown
        .pulls
        .iter()
        .find(|key| Some(key.request.request_id.as_str()) == request(&run.ui))
        .and_then(|key| {
            run.ui
                .placed
                .pulled
                .display(key)
                .and_then(close_action)
                .map(|action| (key.session.clone(), action))
        });
    let now = run.detection.now();
    if crate::sessions::active(&run.ui) {
        crate::sessions::close(&mut run.ui, now);
    } else {
        crate::palette::close(&mut run.ui, now);
    }
    if run.session.tree.viewed().busy() {
        let effects = input::on_key(&mut run.ui, &run.session.tree, key, now);
        run.apply(effects);
    }
    // Interrupt discards queued actions, so enqueue retirement after it.
    if let Some((session, action)) = close {
        send_close(run, session, action);
    }
    true
}

/// An in-flight drawing can finish after close and allocate its cursor then.
pub(super) fn retired(run: &mut Run, key: &crate::pulled::Key, view: &View) -> bool {
    if !matches!(
        key.request.component,
        bingo_sdk::render::Component::Palette | bingo_sdk::render::Component::Sessions
    ) || ((crate::palette::active(&run.ui) || crate::sessions::active(&run.ui))
        && request(&run.ui) == Some(key.request.request_id.as_str()))
    {
        return false;
    }
    if let Some(action) = close_action(view) {
        send_close(run, key.session.clone(), action);
    }
    true
}

fn close_action(view: &View) -> Option<bingo_sdk::Action> {
    let View::Box { layout, .. } = view else {
        return None;
    };
    let mut action = layout.on_key.clone()?;
    if !action.args.is_object() {
        return None;
    }
    action.args["value"] = serde_json::json!({"key":"ctrl+c"});
    Some(action)
}

fn send_close(run: &mut Run, session: SessionId, action: bingo_sdk::Action) {
    if let Some(handle) = run.session.handles.get(&session).cloned() {
        let intent = run.mint(None);
        // Retirement belongs to this drawing, even after its site has closed.
        run.ui.actions.push(session, handle, intent, action, None);
        run.release_actions();
    }
}

pub(super) fn rendered(run: &mut Run, epoch: u64, key: crate::pulled::Key, view: View) {
    if retired(run, &key, &view) {
        return;
    }
    run.ui.placed.pulled.answer_at(epoch, key, view);
    run.release_actions();
    site_keys::drain(run);
}
