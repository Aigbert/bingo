use crate::clock::Now;
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
    out
}
pub const ID: &str = "InfoNotice";
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
        ID,
        props,
        width,
        &crate::views::Marks::at(now),
        |props, width| crate::transcript::component_closing(state, props, width),
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
            && let Some(key) = super::key(state, ID, &props, width, rows)
        {
            frame.pulls.push(key);
        }
        from = to;
    }
}
