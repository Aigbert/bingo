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
            look: Theme {
                colors: theme::Colors::Ansi,
                glyphs: &theme::UNICODE,
            },
            graphics: Graphics::Off,
            motion: true,
        }
    }
}

impl Render {
    pub fn scope<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = CURRENT.with(|slot| slot.replace(Some(*self)));
        let _restore = Restore(previous);
        f()
    }
}

thread_local! {
    static CURRENT: std::cell::Cell<Option<Render>> = const { std::cell::Cell::new(None) };
}

struct Restore(Option<Render>);
impl Drop for Restore {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.set(self.0));
    }
}

pub fn current() -> Render {
    CURRENT.with(std::cell::Cell::get).unwrap_or_default()
}

/// Environment and terminal answers retained by one run for late replies.
#[derive(Clone, Debug)]
pub struct Detection {
    pub ask: theme::Ask,
    pub ground: Option<(u16, u16, u16)>,
    pub heard: Option<crate::graphics::Heard>,
    pub motion: bool,
}

impl Default for Detection {
    fn default() -> Self {
        Self {
            ask: theme::Ask::default(),
            ground: None,
            heard: None,
            motion: true,
        }
    }
}

#[cfg(test)]
impl From<Render> for Detection {
    fn from(render: Render) -> Self {
        use crate::graphics::{Heard, Passthrough, Probe};
        Self {
            ask: theme::Ask {
                no_color: render.look.colors == theme::Colors::Plain,
                indexed: matches!(render.look.colors, theme::Colors::Indexed(_)),
                truecolor: matches!(render.look.colors, theme::Colors::True(_)),
                ascii: render.look.glyphs == &theme::ASCII,
                look: theme::Look::Terminal,
            },
            ground: Some(theme::reference_ground(
                matches!(render.look.colors, theme::Colors::True(palette) | theme::Colors::Indexed(palette) if palette == theme::LIGHT),
            )),
            heard: match render.graphics {
                Graphics::Off => None,
                Graphics::Kitty { cell, transport } => Some(Heard {
                    probe: Probe {
                        kitty: true,
                        cell: Some(cell),
                        terminals: crate::graphics::probe::parse(
                            b"\x1bP>|kitty(0.40.0)\x1b\\\x1bP>|tmux(3.6)\x1b\\",
                        )
                        .terminals,
                    },
                    transport,
                    passthrough: Passthrough::All,
                    measured: false,
                }),
            },
            motion: render.motion,
        }
    }
}

impl Detection {
    pub fn render(&self) -> Render {
        Render {
            look: theme::detect(self.ask, self.ground),
            graphics: crate::graphics::detect(self.heard.as_ref()).graphics,
            motion: self.motion,
        }
    }
    pub fn notice(&self) -> Option<&'static str> {
        crate::graphics::detect(self.heard.as_ref()).notice
    }
    pub fn reply(&mut self, reply: &[u8]) {
        if let Some(ground) = theme::answered_ground(reply) {
            self.ground = Some(ground);
        }
        if let Some(heard) = &mut self.heard {
            heard.and(crate::graphics::probe::parse(reply));
        }
    }
    pub fn now(&self) -> crate::clock::Now {
        crate::clock::Now {
            motion: self.motion,
            ..crate::clock::Now::real()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_detection_uses_environment_and_probe_replies() {
        use crate::graphics::{Cell, Heard, Passthrough, Probe, Transport};
        for (ask, light, colors) in [
            (theme::Ask::default(), None, theme::Colors::Ansi),
            (
                theme::Ask {
                    truecolor: true,
                    ..Default::default()
                },
                Some(true),
                theme::Colors::True(theme::LIGHT),
            ),
            (
                theme::Ask {
                    truecolor: true,
                    look: theme::Look::Dark,
                    ..Default::default()
                },
                Some(true),
                theme::Colors::True(theme::DARK),
            ),
            (
                theme::Ask {
                    no_color: true,
                    truecolor: true,
                    ..Default::default()
                },
                Some(true),
                theme::Colors::Plain,
            ),
        ] {
            let detection = Detection {
                ask,
                ground: light.map(theme::reference_ground),
                motion: false,
                heard: None,
            };
            assert_eq!(detection.render().look.colors, colors);
            assert!(!detection.render().motion);
        }
        let mut detection = Detection {
            ask: theme::Ask {
                truecolor: true,
                ascii: true,
                ..Default::default()
            },
            heard: Some(Heard {
                probe: Probe::default(),
                transport: Transport::Bare,
                passthrough: Passthrough::Unknown,
                measured: false,
            }),
            ..Default::default()
        };
        assert_eq!(detection.render().graphics, Graphics::Off);
        detection.reply(b"\x1b_Gi=31;OK\x1b\\\x1b[6;16;8t\x1bP>|kitty(0.40.0)\x1b\\");
        assert_eq!(
            detection.render().graphics,
            Graphics::Kitty {
                cell: Cell {
                    width: 8,
                    height: 16
                },
                transport: Transport::Bare
            }
        );
        detection.reply(b"\x1b]11;rgb:ffff/ffff/ffff\x07");
        assert_eq!(
            detection.render().look.colors,
            theme::Colors::True(theme::LIGHT)
        );
        assert_eq!(detection.render().look.glyphs, &theme::ASCII);
        detection.reply(b"\x1b]11;rgb:0000/0000/0000\x07");
        assert_eq!(
            detection.render().look.colors,
            theme::detect(detection.ask, Some((0, 0, 0))).colors
        );
    }

    #[test]
    fn render_scopes_isolate_parallel_draws_and_restore_after_unwind() {
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|threads| {
            for look in [crate::painted::daylight(), crate::painted::truecolor()] {
                let barrier = &barrier;
                threads.spawn(move || {
                    let render = Render {
                        look,
                        ..Render::default()
                    };
                    render.scope(|| {
                        barrier.wait();
                        assert_eq!(current(), render);
                        assert_eq!(theme::current(), look);
                        Render::default().scope(|| assert_eq!(current(), Render::default()));
                        assert_eq!(current(), render);
                        let mut terminal =
                            ratatui::Terminal::new(ratatui::backend::TestBackend::new(5, 1))
                                .unwrap();
                        terminal
                            .draw(|frame| {
                                frame.render_widget(
                                    ratatui::widgets::Paragraph::new("test").style(theme::raised()),
                                    frame.area(),
                                )
                            })
                            .unwrap();
                        assert_eq!(
                            terminal.backend().buffer()[(0, 0)].bg,
                            match look.colors {
                                theme::Colors::True(palette) => palette.raised,
                                _ => unreachable!(),
                            }
                        );
                    });
                    assert_eq!(current(), Render::default());
                });
            }
        });
        let look = Render {
            look: crate::painted::daylight(),
            ..Render::default()
        };
        let _ = std::panic::catch_unwind(|| look.scope(|| panic!("leave the pass")));
        assert_eq!(current(), Render::default());
    }
}
