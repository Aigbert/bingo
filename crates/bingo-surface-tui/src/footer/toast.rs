use super::ink;
use crate::{
    clock::{self, Now},
    layout::{Layout, grid},
    mode, theme,
    ui::{NoticeKind, TOAST_FADE, Ui},
    views::Drawing,
};
use ratatui::text::{Line, Span};
use std::time::Duration;

pub(super) fn draw(
    state: &bingo_sdk::SessionState,
    ui: &Ui,
    frame: &mut Layout,
    width: usize,
    now: Now,
) -> Option<Drawing> {
    if width == 0 {
        return None;
    }
    let notice = crate::toast::components(ui, state, now, width, &mut frame.pulls)?;
    let age = now.since(notice.said());
    let hold = notice.kind.hold();
    let mut spans = words(&notice.kind, state);
    fade(&mut spans, age, hold, now.motion);
    Some(Drawing {
        lines: vec![grid::fit(Line::from(spans), width)],
        next: Some(now.instant + next(age, hold, now.motion)),
    })
}

fn words(kind: &NoticeKind, state: &bingo_sdk::SessionState) -> Vec<Span<'static>> {
    match kind {
        NoticeKind::Mode => {
            let mode = mode::current(state);
            let mut spans = vec![Span::styled(
                mode.label.clone(),
                ink(mode.flash_ink()).patch(theme::bold()),
            )];
            if let Some(meaning) = mode.meaning {
                spans.push(Span::styled(theme::glyphs::get("sep"), ink("faint")));
                spans.push(Span::styled(meaning, ink("muted")));
            }
            spans
        }
        NoticeKind::Message(toast) => {
            let color = match toast.level {
                bingo_sdk::Level::Error => "bad",
                bingo_sdk::Level::Warn => "amber",
                _ => "fg",
            };
            let mut spans = vec![Span::styled(toast.text.clone(), ink(color))];
            if let Some(about) = &toast.about {
                spans.push(Span::styled(format!(" · {about}"), ink("dim")));
            }
            spans
        }
    }
}

fn fade(spans: &mut [Span<'_>], age: Duration, hold: Duration, motion: bool) {
    let weight = if motion {
        (age.as_secs_f64() / 0.160)
            .min(1.0)
            .min(1.0 - age.saturating_sub(hold).as_secs_f64() / TOAST_FADE.as_secs_f64())
            .max(0.0)
    } else {
        1.0
    };
    for span in spans {
        if let Some(color) = span.style.fg {
            let bg = theme::palette().named("bg").unwrap_or(color);
            span.style = span
                .style
                .patch(theme::literal(theme::ink::mix(bg, color, weight)));
        }
    }
}

fn next(age: Duration, hold: Duration, motion: bool) -> Duration {
    let end = hold + TOAST_FADE;
    let target = if !motion {
        end
    } else if age < Duration::from_millis(160) || age >= hold {
        age + clock::FRAME
    } else {
        hold
    };
    target.min(end).saturating_sub(age)
}
