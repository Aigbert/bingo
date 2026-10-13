//! The value a rendering pass runs in.

use crate::graphics::Graphics;
use crate::theme::{self, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Render {
    pub look: Theme,
    pub graphics: Graphics,
    pub motion: bool,
}

impl Default for Render {
    fn default() -> Self {
        Self {
            look: Theme { colors: theme::Colors::Ansi, glyphs: &theme::UNICODE },
            graphics: Graphics::Off,
            motion: true,
        }
    }
}

impl Render {
    pub fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
        f()
    }
}

pub fn current() -> Render {
    Render::default()
}

/// Environment and terminal answers retained by one run for late replies.
#[derive(Clone, Debug, Default)]
pub struct Detection {
    pub ask: theme::Ask,
    pub light: Option<bool>,
    pub heard: Option<crate::graphics::Heard>,
    pub motion: bool,
}

impl Detection {
    pub fn render(&self) -> Render { Render::default() }
    pub fn reply(&mut self, _reply: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_detection_uses_environment_and_probe_replies() {
        use crate::graphics::{Cell, Heard, Passthrough, Probe, Transport};
        for (ask, light, colors) in [
            (theme::Ask::default(), None, theme::Colors::Ansi),
            (theme::Ask { truecolor: true, ..Default::default() }, Some(true), theme::Colors::True(theme::LIGHT)),
            (theme::Ask { truecolor: true, look: theme::Look::Dark, ..Default::default() }, Some(true), theme::Colors::True(theme::DARK)),
            (theme::Ask { no_color: true, truecolor: true, ..Default::default() }, Some(true), theme::Colors::Plain),
        ] {
            let detection = Detection { ask, light, motion: false, heard: None };
            assert_eq!(detection.render().look.colors, colors);
            assert!(!detection.render().motion);
        }
        let mut detection = Detection {
            ask: theme::Ask { truecolor: true, ascii: true, ..Default::default() },
            heard: Some(Heard { probe: Probe::default(), transport: Transport::Bare, passthrough: Passthrough::Unknown }),
            ..Default::default()
        };
        assert_eq!(detection.render().graphics, Graphics::Off);
        detection.reply(b"\x1b_Gi=31;OK\x1b\\\x1b[6;16;8t\x1bP>|kitty(0.40.0)\x1b\\");
        assert_eq!(detection.render().graphics, Graphics::Kitty { cell: Cell { width: 8, height: 16 }, transport: Transport::Bare });
        detection.reply(b"\x1b]11;rgb:ffff/ffff/ffff\x07");
        assert_eq!(detection.render().look.colors, theme::Colors::True(theme::LIGHT));
        assert_eq!(detection.render().look.glyphs, &theme::ASCII);
        detection.reply(b"\x1b]11;rgb:0000/0000/0000\x07");
        assert_eq!(detection.render().look.colors, theme::Colors::True(theme::DARK));
    }

    #[test]
    fn render_scopes_isolate_parallel_draws_and_restore_after_unwind() {
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|threads| {
            for look in [crate::painted::daylight(), crate::painted::truecolor()] {
                let barrier = &barrier;
                threads.spawn(move || {
                    let render = Render { look, ..Render::default() };
                    render.scope(|| {
                        barrier.wait();
                        assert_eq!(current(), render);
                        assert_eq!(theme::current(), look);
                        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(5, 1)).unwrap();
                        terminal.draw(|frame| frame.render_widget(ratatui::widgets::Paragraph::new("test").style(theme::raised()), frame.area())).unwrap();
                        assert_eq!(terminal.backend().buffer()[(0, 0)].bg, match look.colors {
                            theme::Colors::True(palette) => palette.raised,
                            _ => unreachable!(),
                        });
                    });
                    assert_eq!(current(), Render::default());
                });
            }
        });
        let look = Render { look: crate::painted::daylight(), ..Render::default() };
        let _ = std::panic::catch_unwind(|| look.scope(|| panic!("leave the pass")));
        assert_eq!(current(), Render::default());
    }
}
