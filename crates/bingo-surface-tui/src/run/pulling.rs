use super::*;

pub(super) fn sync(run: &mut Run, screen: &mut dyn Screen, now: Now) -> Result<(), KernelError> {
    let state = run.session.tree.viewed();
    run.ui.sync_panes(state);
    let holding = run.ui.placed.panes.holding(state);
    if run.ui.placed.toasts.sync(state, holding, now) && !run.ui.focused {
        screen.bell().map_err(stdio)?;
    }
    Ok(())
}

pub(super) fn request(run: &mut Run, keys: &[crate::pulled::Key]) -> bool {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    let mut answered = false;
    run.ui.placed.pulled.retain(keys);
    for key in keys {
        if !run.ui.placed.pulled.request(key) {
            continue;
        }
        let epoch = run.ui.placed.pulled.epoch();
        let key = key.clone();
        let host = run.host.clone();
        let mut pending = Box::pin(async move {
            let view = host
                .render(&key.session, key.request.clone())
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
    let began = Instant::now();
    let (width, height) = screen.size();
    pulling::sync(run, screen, now)?;
    run.ui.prepare(&run.session.tree.viewed().summary.cwd);
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
    pulling::request(run, &layout.pulls);
    run.ui.shown = layout;
    site_keys::drain(run);
    run.grumble(began.elapsed());
    showing::hand(run, screen)?;
    showing::fit(run);
    showing::read_linked(run);
    Ok(())
}
