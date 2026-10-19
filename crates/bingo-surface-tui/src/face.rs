//! A name's stable visual identity; the fixture is shared with wire clients.
use crate::{clock, theme};
use ratatui::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face(pub u32);

/// FNV-1a 32-bit over the exact UTF-8 name; no theme or stored identity.
pub fn face(name: &str) -> Face {
    Face(bingo_sdk::view::identity::hash(name))
}

impl Face {
    pub fn hue(self) -> usize {
        bingo_sdk::view::identity::hue(self.0)
    }

    pub fn sigil(self) -> &'static str {
        let glyphs = if theme::glyphs() == &theme::ASCII {
            ["@", "#", "%", "&", "+", "="]
        } else {
            ["◆", "●", "▲", "■", "◗", "✦"]
        };
        glyphs[(self.0 % 6) as usize]
    }
}

pub fn style(name: &str) -> Style {
    theme::crew(face(name).hue())
}

/// Weight pulses without ever borrowing a state colour for an identity.
pub fn pulse(name: &str, live: bool, now: clock::Now) -> Style {
    let style = style(name);
    if live && now.motion && clock::alternating(now, theme::PULSE) {
        style.patch(theme::bold())
    } else {
        style
    }
}
