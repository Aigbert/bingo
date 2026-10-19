use super::*;
fn request(ui: &Ui) -> Option<&str> {
    crate::sessions::request(ui)
        .or_else(|| crate::browse::request(ui))
        .or_else(|| crate::plugins::request(ui))
        .or_else(|| crate::palette::request(ui))
}

/// Whether one of the drawn sites that Ctrl+C closes is open; Esc is its own.
pub(super) fn open(ui: &Ui) -> bool {
    crate::palette::active(ui)
        || crate::sessions::active(ui)
        || crate::browse::active(ui)
        || crate::plugins::active(ui)
}

/// Closing consumes Ctrl+C at idle; a running turn still gets its interrupt.
pub(super) fn ctrl_c(run: &mut Run, key: crossterm::event::KeyEvent) -> bool {
    if !open(&run.ui) {
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
    let now = run.now();
    if crate::sessions::active(&run.ui) {
        crate::sessions::close(&mut run.ui, now);
    } else if crate::browse::active(&run.ui) {
        run.ui.focus.close(now.instant);
    } else if crate::plugins::active(&run.ui) {
        crate::plugins::close(&mut run.ui, now);
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

/// Whether a drawing belongs to the site still open. Browse keeps its state
/// while its pager is up, so its drawing stays current while it is kept.
fn current(ui: &Ui, id: &str) -> bool {
    (open(ui) && request(ui) == Some(id)) || crate::browse::request(ui) == Some(id)
}

/// An in-flight drawing can finish after close and allocate its cursor then.
pub(super) fn retired(run: &mut Run, key: &crate::pulled::Key, view: &View) -> bool {
    if !matches!(
        key.request.component,
        bingo_sdk::render::Component::Palette
            | bingo_sdk::render::Component::Sessions
            | bingo_sdk::render::Component::Browse
            | bingo_sdk::render::Component::Plugins
    ) || current(&run.ui, &key.request.request_id)
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
    layout
        .on_key
        .clone()?
        .with_value(serde_json::json!({"key":"ctrl+c"}))
}

fn send_close(run: &mut Run, session: SessionId, action: bingo_sdk::Action) {
    if let Some(handle) = run.session.handles.get(&session).cloned() {
        let intent = run.mint(crate::run::Submission::Ordinary(None));
        // Retirement belongs to this drawing, even after its site has closed.
        let origin = crate::pulled_signature(&action.name);
        run.ui
            .actions
            .push(session, handle, intent, (action, origin), None);
        run.release_actions();
    }
}

pub(super) fn rendered(run: &mut Run, epoch: u64, key: crate::pulled::Key, view: View) {
    if retired(run, &key, &view) {
        return;
    }
    run.ui.placed.pulled.answer_at(epoch, key, view);
    run.release_actions();
    typeahead::drain(run);
}
