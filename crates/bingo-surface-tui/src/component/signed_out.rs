//! An expired sign-in ends its turn with what happened and what to do.
use crate::views;
use bingo_sdk::{
    ErrorCode, SessionState, TurnStatus, View,
    component::{InfoNoticeProps, Props},
    pick,
};
use ratatui::text::Line;

/// The command the second row names; the closing's props carry it.
pub const COMMAND: &str = "login";

/// The closing's props when the last turn was refused for its credentials.
pub fn props(state: &SessionState) -> Option<Props> {
    let Some(TurnStatus::Failed { error }) = state.last_status() else {
        return None;
    };
    (error.code == ErrorCode::AuthRequired).then(|| {
        Props::InfoNotice(InfoNoticeProps {
            text: error.message.clone(),
            command: Some(COMMAND.into()),
        })
    })
}

pub fn lines(state: &SessionState, message: &str, width: usize) -> Vec<Line<'static>> {
    let sep = || pick::text(crate::theme::glyphs::get("sep"), "faint");
    let head = match &state.summary.provider {
        Some(provider) => format!("signed out of {provider}"),
        None => "signed out".into(),
    };
    let view = View::Box {
        children: vec![
            row(pick::line([
                pick::text(head, "bad"),
                pick::text(crate::theme::glyphs::get("sep"), "dim"),
                pick::text(message, "fg"),
            ])),
            row(pick::line([
                pick::text(format!("/{COMMAND}"), "glow"),
                pick::text(" signs you in again", "muted"),
                sep(),
                pick::text("your message is kept", "muted"),
            ])),
        ],
        layout: Default::default(),
    };
    views::marked(&view, width, &views::Marks::default()).lines
}

fn row(line: View) -> View {
    pick::line([pick::text("   ", "fg"), line])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::KernelError;

    fn failed(code: ErrorCode) -> SessionState {
        let error = KernelError::new(code, "the token expired");
        let mut state = folded(vec![
            frame(1, started("t1")),
            frame(2, completed("t1", TurnStatus::Failed { error })),
        ]);
        state.summary.provider = Some("anthropic".into());
        state
    }

    #[test]
    fn an_expired_sign_in_says_what_happened_then_what_to_do() {
        let state = failed(ErrorCode::AuthRequired);
        let Some(Props::InfoNotice(props)) = props(&state) else {
            panic!("signed out")
        };
        let rows: Vec<String> = lines(&state, &props.text, 80)
            .iter()
            .map(|line| line.to_string().trim_end().to_owned())
            .collect();
        assert_eq!(
            rows,
            [
                "   signed out of anthropic · the token expired",
                "   /login signs you in again · your message is kept",
            ]
        );
    }

    #[test]
    fn any_other_failure_is_not_signed_out() {
        assert!(props(&failed(ErrorCode::ProviderUnavailable)).is_none());
    }
}
