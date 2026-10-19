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
    let notice = crate::toast::components(ui, state, now, width, &mut frame.pulls)?;
    let age = now.since(notice.said());
    let hold = notice.hold();
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
            let mut spans =
                crate::tone::parts(&toast.parts, toast.tone, crate::tone::Content::Plain, None);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_toast_uses_its_tone_once_and_keeps_its_tail_dim() {
        let kind = NoticeKind::Message(crate::ui::Message {
            parts: vec!["failed · literally".into(), "try again".into()],
            tone: bingo_sdk::Tone::Bad,
            about: Some("draft".into()),
        });
        let spans = words(&kind, &crate::test_support::state());
        assert_eq!(spans[0].style, ink("bad"));
        assert_eq!(
            spans[0].content,
            format!("{} ", theme::glyphs::get("times"))
        );
        assert_eq!(spans[1].content, "failed · literally");
        assert_eq!(spans[1].style, ink("muted"));
        assert_eq!(spans[2].style, ink("faint"));
        assert_eq!(spans[3].style, ink("muted"));
        assert_eq!(spans[4].style, ink("dim"));
    }
}
