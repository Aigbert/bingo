use super::*;

pub(super) fn sync(run: &mut Run, screen: &mut dyn Screen, now: Now) -> Result<(), KernelError> {
    rewind_files(run);
    let state = run.session.tree.viewed();
    run.ui.sync_panes(state);
    if run.ui.sync_toasts(state, now) && !run.ui.focused {
        screen.bell().map_err(stdio)?;
    }
    Ok(())
}

pub(super) fn request(run: &mut Run, keys: &[crate::pulled::Key]) -> bool {
    run.ui.placed.pulled.retain(keys);
    ask(run, keys)
}

/// A drawing a key waits on is asked for again the moment it is
/// invalidated, not at the next paint: a held arrow otherwise pays a frame
/// boundary per press and falls behind the key repeat.
pub(super) fn refetch(run: &mut Run, requests: &[String]) {
    if run.site_keys.is_empty() && run.typeahead.is_empty() {
        return;
    }
    let keys = run
        .ui
        .shown
        .pulls
        .iter()
        .filter(|key| requests.contains(&key.request.request_id))
        .filter_map(|key| {
            let state = run.session.tree.state(&key.session)?;
            let viewport = &key.request.viewport;
            Some(crate::pulled::Key::new(
                state,
                &key.request,
                viewport.columns,
                viewport.rows,
            ))
        })
        .collect::<Vec<_>>();
    if ask(run, &keys) {
        run.release_actions();
        typeahead::drain(run);
    }
}

fn ask(run: &mut Run, keys: &[crate::pulled::Key]) -> bool {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    let mut answered = false;
    for key in keys {
        if !run.ui.placed.pulled.request(key) {
            continue;
        }
        let epoch = run.ui.placed.pulled.epoch();
        let key = key.clone();
        let host = run.host.clone();
        let request = resume_request(&key, run.ui.placed.pulled.display(&key));
        let mut pending = Box::pin(async move {
            let view = host
                .render(&key.session, request)
                .await
                .unwrap_or_else(|_| key.request.engine());
            Ok(Reply::Rendered(epoch, key, Box::new(view)))
        });
        match pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(Ok(Reply::Rendered(epoch, key, view))) => {
                run.ui.placed.pulled.answer_at(epoch, key, *view);
                answered = true;
            }
            Poll::Pending => run.spawn(pending),
            _ => {}
        }
    }
    answered
}

pub(super) fn paint(
    run: &mut Run,
    screen: &mut dyn Screen,
    wake: Wake,
    now: Now,
) -> Result<(), KernelError> {
    if wake == Wake::Event && now.since(run.painted) < TICK {
        run.behind = true;
        return Ok(());
    }
    run.behind = false;
    run.painted = now.instant;
    run.hand_over(screen)?;
    look::pay(run, screen).map_err(stdio)?;
    let title = run.detection.render().scope(|| title(&run.session.tree));
    screen.title(&title).map_err(stdio)?;
    // Measure host-call latency on real time, independently of the frame clock.
    let began = Instant::now();
    let (width, height) = screen.size();
    pulling::sync(run, screen, now)?;
    run.ui.draws_notice(now.instant);
    run.ui.prepare(&run.session.tree.viewed().summary.cwd);
    read_changes(run);
    let previous = std::mem::take(&mut run.ui.shown);
    let mut layout = crate::layout::layout(
        previous,
        &run.session.tree,
        &run.ui,
        ratatui::layout::Rect::new(0, 0, width, height),
        now,
        &run.detection.render(),
    );
    if request(run, &layout.pulls) {
        layout = crate::layout::layout(
            layout,
            &run.session.tree,
            &run.ui,
            ratatui::layout::Rect::new(0, 0, width, height),
            now,
            &run.detection.render(),
        );
    }
    screen
        .draw(&layout, &run.detection.render())
        .map_err(stdio)?;
    run.behind |= pulling::request(run, &layout.pulls);
    run.ui.shown = layout;
    run.release_actions_at(now.instant);
    typeahead::drain(run);
    let preview = crate::browse::preview(&mut run.ui, &run.session.tree);
    run.apply(preview);
    run.grumble(began.elapsed());
    showing::hand(run, screen)?;
    showing::fit(run);
    showing::read_linked(run);
    Ok(())
}

fn resume_request(
    key: &crate::pulled::Key,
    previous: Option<&bingo_sdk::View>,
) -> bingo_sdk::render::RenderRequest {
    let mut request = key.request.clone();
    if request.component == bingo_sdk::render::Component::Status
        && let Some(bingo_sdk::View::Box { layout, .. }) = previous
        && let Some(action) = &layout.on_key
        && let Some(selected) = action.args["selected"].as_str()
        && let Some(props) = request.props.as_object_mut()
    {
        props.insert("selected".into(), selected.into());
    }
    request
}

fn rewind_files(run: &mut Run) {
    let Some(service) = bingo_services::checkpoints::find(&run.host) else {
        return;
    };
    let Some(Open::Rewind(card)) = run.ui.focus.open_mut() else {
        return;
    };
    let session = run.session.tree.view().clone();
    let turns = bingo_sdk::transcript::turns(&run.session.tree.viewed().transcript);
    let missing = turns
        .into_iter()
        .filter_map(|turn| {
            if card.touched.contains_key(&turn.id) {
                return None;
            }
            card.touched.insert(turn.id.clone(), None);
            Some(turn.id)
        })
        .collect::<Vec<_>>();
    let request = card.request.clone();
    for turn in missing {
        let request = request.clone();
        let (service, session) = (service.clone(), session.clone());
        run.spawn(async move {
            let touched = service.touched(&session, &turn).await.ok().flatten();
            Ok(Reply::RewindFiles(TurnFiles {
                request,
                session,
                turn,
                touched,
            }))
        });
    }
}

/// What one turn of the rewind card changed, as the checkpoints owner told
/// it for the card that asked.
pub(super) struct TurnFiles {
    pub(super) request: IntentId,
    pub(super) session: SessionId,
    pub(super) turn: bingo_sdk::TurnId,
    pub(super) touched: Option<bingo_services::checkpoints::Touched>,
}

/// A turn's files land on the card only while that card is still the one
/// open over the session that asked.
pub(super) fn turn_files(run: &mut Run, files: TurnFiles) {
    if &files.session == run.session.tree.view()
        && let Some(Open::Rewind(card)) = run.ui.focus.open_mut()
        && card.request == files.request
    {
        card.touched.insert(files.turn, files.touched);
    }
}

/// The `@` list opened: read git's word on its files off this thread.
fn read_changes(run: &mut Run) {
    if let Some((opening, cwd)) = run.ui.files.ask() {
        run.spawn(async move {
            let changes = crate::git_changes::read(cwd, crate::git_changes::DEADLINE).await;
            Ok(super::Reply::Changes(opening, changes))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::render::{Component, RenderRequest, SurfaceName, Viewport};
    use serde_json::json;
    #[test]
    fn a_status_refresh_carries_the_displayed_selection() {
        let state = crate::test_support::state();
        let request = RenderRequest {
            component: Component::Status,
            request_id: "status".into(),
            surface: SurfaceName::Terminal,
            props: json!({}),
            viewport: Viewport {
                glyphs: Default::default(),
                place: None,
                columns: 80,
                rows: 12,
            },
        };
        let key = crate::pulled::Key::new(&state, &request, 80, 12);
        let view: bingo_sdk::View = serde_json::from_value(json!({"kind":"Box","children":[],
            "onKey":{"name":"fixture:ui","args":{"selected":"plugins"}}}))
        .unwrap();
        assert_eq!(
            resume_request(&key, Some(&view)).props["selected"],
            "plugins"
        );
        assert_eq!(key.request.props["selected"], serde_json::Value::Null);
    }
}
