//! A name's stable visual identity; the fixture is shared with wire clients.
use crate::{clock, theme};
use ratatui::style::{Modifier, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    pub hash: u32,
    pub hue: usize,
    pub sigil: &'static str,
}

pub fn face(name: &str) -> Face {
    let hash = name.bytes().fold(2_166_136_261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    let glyphs = if theme::glyphs() == &theme::ASCII {
        ["@", "#", "%", "&", "+", "="]
    } else {
        ["◆", "●", "▲", "■", "◗", "✦"]
    };
    Face {
        hash,
        hue: ((hash >> 8) % 5) as usize,
        sigil: glyphs[(hash % 6) as usize],
    }
}

pub fn style(name: &str) -> Style {
    theme::crew(face(name).hue)
}

/// Weight pulses without ever borrowing a state colour for an identity.
pub fn pulse(name: &str, live: bool, now: clock::Now) -> Style {
    let style = style(name);
    if live && now.motion && clock::alternating(now, theme::PULSE) {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}
