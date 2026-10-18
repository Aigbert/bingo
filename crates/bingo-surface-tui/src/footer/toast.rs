use super::ink;
use crate::{
    clock::{self, Now},
    layout::{Layout, grid},
    mode, theme,
    ui::Ui,
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
    let toast = crate::toast::components(ui, state, now, width, &mut frame.pulls)?;
    let age = crate::toast::age(ui, state, now)?;
    let mode = toast
        .text
        .starts_with("permission mode: ")
        .then(|| mode::changed(state, ui, now))
        .flatten()
        .map(|_| mode::current(state));
    let hold = if mode.is_some() { 2800 } else { 2400 };
    let mut spans = if let Some(mode) = mode {
        vec![
            Span::styled(
                mode.label.clone(),
                ink(mode.flash_ink()).patch(theme::bold()),
            ),
            Span::styled(theme::glyphs::get("sep"), ink("faint")),
            Span::styled(mode.description(), ink("muted")),
        ]
    } else {
        let color = match toast.level {
            bingo_sdk::Level::Error => "bad",
            bingo_sdk::Level::Warn => "amber",
            _ => "fg",
        };
        let mut spans = vec![Span::styled(toast.text, ink(color))];
        if let Some(about) = toast.about {
            spans.push(Span::styled(format!(" · {about}"), ink("dim")));
        }
        spans
    };
    let ms = age.as_secs_f64() * 1000.0;
    let k = if now.motion {
        (ms / 160.0)
            .min(1.0)
            .min(1.0 - ((ms - hold as f64) / 300.0).clamp(0.0, 1.0))
    } else {
        1.0
    };
    for span in &mut spans {
        if let Some(color) = span.style.fg {
            let bg = theme::palette().named("bg").unwrap_or(color);
            span.style = span
                .style
                .patch(theme::literal(theme::ink::mix(bg, color, k)));
        }
    }
    let end = Duration::from_millis(hold + 300);
    let next = if !now.motion {
        end
    } else if ms < 160.0 || ms >= hold as f64 {
        age + clock::FRAME
    } else if ms < hold as f64 {
        Duration::from_millis(hold)
    } else {
        end
    };
    Some(Drawing {
        lines: vec![grid::fit(Line::from(spans), width)],
        next: Some(now.instant + next.min(end).saturating_sub(age)),
    })
}
