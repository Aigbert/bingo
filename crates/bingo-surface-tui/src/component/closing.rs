use crate::{clock::Now, worked};
use bingo_sdk::{SessionState, TurnStatus, component::*};

pub fn props(state: &SessionState) -> Vec<Props> {
    if state.busy() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Some(TurnStatus::Failed { error }) = state.last_status() {
        out.push(Props::InfoNotice(InfoNoticeProps {
            text: error.message.clone(),
            command: None,
        }));
    }
    if let Some(turn) = &state.last_turn
        && worked::ran(turn) >= worked::SAY_AFTER
    {
        out.push(Props::TurnDuration(TurnDurationProps {
            word: worked::verb(&turn.status).0.into(),
            duration_ms: worked::ran(turn).as_millis().min(u128::from(u64::MAX)) as u64,
        }));
    }
    out
}
pub fn id(props: &Props) -> &'static str {
    match props {
        Props::InfoNotice(_) => "InfoNotice",
        _ => "TurnDuration",
    }
}
pub fn draw(
    state: &SessionState,
    memo: Option<&crate::pulled::Memo>,
    props: &Props,
    width: usize,
    now: Now,
) -> crate::views::Drawing {
    super::resolve(
        state,
        memo,
        id(props),
        props,
        width,
        &crate::views::Marks::at(now),
        |props, _| crate::transcript::component_closing(state, props, now),
    )
}
pub fn requests(
    state: &SessionState,
    ui: &crate::ui::Ui,
    frame: &mut crate::layout::Layout,
    width: u16,
    rows: u16,
    now: Now,
) {
    let mut from = frame.blocks.closing_start();
    for props in props(state) {
        let to = from
            + draw(state, Some(&ui.placed.pulled), &props, width.into(), now)
                .lines
                .len();
        if to > frame.top
            && from < frame.top + usize::from(rows)
            && let Some(key) = super::key(state, id(&props), &props, width, rows)
        {
            frame.pulls.push(key);
        }
        from = to;
    }
}
