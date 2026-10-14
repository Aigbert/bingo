use super::*;

/// Read this workspace once for the palette opening that requested it.
pub(super) fn list_stored(run: &mut Run) {
    let Some(request) = crate::palette::request(&run.ui).map(str::to_owned) else {
        return;
    };
    let host = run.host.clone();
    let cwd = Some(run.session.tree.root().summary.cwd.clone().into());
    run.spawn(async move {
        host.sessions(SessionFilter {
            cwd,
            ..Default::default()
        })
        .await
        .map(|sessions| Reply::Stored(request, sessions))
    });
}

/// The store's answer lands after the card is already up. It fills the
/// card while that is still what is open, and is dropped when it is not.
pub(super) fn fill_palette(run: &mut Run, request: &str, mut sessions: Vec<SessionSummary>) {
    if crate::palette::request(&run.ui) != Some(request) {
        return;
    }
    if let Some(Open::Palette(open)) = run.ui.focus.open_mut() {
        sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        open.children = crate::tree::descendants(run.session.tree.root_id(), &sessions)
            .into_iter()
            .filter(|s| run.session.tree.state(&s.id).is_none())
            .take(crate::palette::sessions::RECENT)
            .cloned()
            .collect();
        open.sessions = crate::palette::sessions::recent_roots(sessions);
    }
}

pub(super) fn surface_action(run: &mut Run, action: bingo_sdk::Action) {
    if action.name != "Palette" {
        return;
    }
    let now = run.detection.now();
    if let Some(id) = action.args["resume"].as_str() {
        run.apply(vec![Effect::Open(SessionSelector::ById {
            id: SessionId::from_raw(id),
        })]);
    } else if let Ok(first) = serde_json::from_value(action.args["open"].clone()) {
        let effects = crate::palette::open(&mut run.ui, first, now);
        run.apply(effects);
    }
}

/// Closing consumes Ctrl+C at idle; a running turn still gets its interrupt.
pub(super) fn ctrl_c(run: &mut Run, key: crossterm::event::KeyEvent) -> bool {
    if !crate::palette::active(&run.ui) {
        return false;
    }
    let close = run
        .ui
        .shown
        .pulls
        .iter()
        .find(|key| Some(key.request.request_id.as_str()) == crate::palette::request(&run.ui))
        .and_then(|key| {
            run.ui
                .placed
                .pulled
                .display(key)
                .and_then(close_action)
                .map(|action| (key.session.clone(), action))
        });
    let now = run.detection.now();
    crate::palette::close(&mut run.ui, now);
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
    if key.request.component != bingo_sdk::render::Component::Palette
        || (crate::palette::active(&run.ui)
            && crate::palette::request(&run.ui) == Some(key.request.request_id.as_str()))
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
        run.ui.actions.push(session, handle, intent, action);
    }
}
