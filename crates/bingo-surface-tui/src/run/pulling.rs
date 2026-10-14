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

pub(super) fn request(run: &mut Run, keys: &[crate::pulled::Key]) {
    run.ui.placed.pulled.retain(keys);
    for key in keys {
        if run.ui.placed.pulled.request(key) {
            let key = key.clone();
            let host = run.host.clone();
            run.spawn(async move {
                let view = host
                    .render(&key.session, key.request.clone())
                    .await
                    .unwrap_or_else(|_| key.request.engine());
                Ok(Reply::Rendered(key, Box::new(view)))
            });
        }
    }
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
    let layout = crate::layout::layout(
        previous,
        &run.session.tree,
        &run.ui,
        ratatui::layout::Rect::new(0, 0, width, height),
        now,
        &run.detection.render(),
    );
    screen
        .draw(&layout, &run.detection.render())
        .map_err(stdio)?;
    pulling::request(run, &layout.pulls);
    run.ui.shown = layout;
    run.grumble(began.elapsed());
    showing::hand(run, screen)?;
    showing::fit(run);
    showing::read_linked(run);
    Ok(())
}
