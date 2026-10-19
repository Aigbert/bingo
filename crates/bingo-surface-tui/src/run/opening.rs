//! Arrival toasts and departure facts at the host boundary.
use super::Run;

/// A resumed session is toasted; a new one starts the welcome's clock.
pub(super) fn arrived(run: &mut Run, reopened: bool) {
    if reopened {
        super::surface::resumed(run);
    } else {
        run.ui.welcome.started_at_ms = Some(run.now().wall.as_millisecond());
    }
}

pub(super) async fn receipt(
    run: &mut Run,
    now: crate::clock::Now,
    width: u16,
) -> Vec<ratatui::text::Line<'static>> {
    let state = run.session.tree.root();
    let facts = crate::receipt::facts(state, now.wall, &crate::tool_rows::Looks::in_ui(&run.ui));
    if facts.turns == 0 {
        return vec![];
    }
    let props = crate::receipt::props(state, facts);
    let request = bingo_sdk::render::RenderRequest {
        component: props.component(),
        request_id: "Receipt".into(),
        surface: bingo_sdk::render::SurfaceName::Terminal,
        props: props.value(),
        viewport: bingo_sdk::render::Viewport {
            glyphs: Default::default(),
            place: None,
            columns: width,
            rows: 3,
        },
    };
    let answer = run.host.render(&state.summary.id, request).await.ok();
    run.detection
        .render()
        .scope(|| crate::receipt::draw(&props, answer.as_ref(), width.into()))
}
