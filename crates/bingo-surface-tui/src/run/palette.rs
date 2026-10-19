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
    let now = run.now();
    if let Some(id) = action.args["resume"].as_str() {
        run.apply(vec![Effect::Open(SessionSelector::ById {
            id: SessionId::from_raw(id),
        })]);
    } else if let Ok(first) = serde_json::from_value(action.args["open"].clone()) {
        if first == bingo_sdk::component::PaletteOpen::Sessions
            && crate::sessions::exists(run.session.tree.viewed())
        {
            let effects = crate::sessions::open(&mut run.ui, now);
            run.apply(effects);
            return;
        }
        let effects = crate::palette::open(&mut run.ui, first, now);
        run.apply(effects);
    }
}
