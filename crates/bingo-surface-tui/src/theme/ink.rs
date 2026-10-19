//! The prototype palette, with rounding at each sRGB blend.
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    base: Base,
    shades: Shades,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Base {
    bg: Color,
    fg: Color,
    ember: Color,
    glow: Color,
    coal: Color,
    amber: Color,
    amber_glow: Color,
    good: Color,
    bad: Color,
    plan: Color,
    auto: Color,
    code: Color,
    syntax: Syntax,
    crew: [Color; 6],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Syntax {
    pub kw: Color,
    pub string: Color,
    pub num: Color,
    pub function: Color,
    pub ty: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shades {
    pub muted: Color,
    pub dim: Color,
    pub faint: Color,
    pub you: Color,
    pub well: Color,
    pub well_deep: Color,
    pub pick: Color,
    pub pick_bg: Color,
    pub add_tint: Color,
    pub del_tint: Color,
    pub add_word: Color,
    pub del_word: Color,
    pub bad_tint: Color,
    pub amber_tint: Color,
    pub glow_tint: Color,
}

pub const LIGHT: Palette = Palette::new(
    Base {
        bg: rgb(0xf9f4f2),
        fg: rgb(0x3a342e),
        ember: rgb(0xb04c14),
        glow: rgb(0xef7a2c),
        coal: rgb(0xb04c14),
        amber: rgb(0x9a6200),
        amber_glow: rgb(0xe0a01c),
        good: rgb(0x2b7d3c),
        bad: rgb(0xb3261e),
        plan: rgb(0x2f62a8),
        auto: rgb(0x7445c0),
        code: rgb(0x8a5520),
        syntax: Syntax {
            kw: rgb(0x8a3fa8),
            string: rgb(0x4d7a1c),
            num: rgb(0xb04c14),
            function: rgb(0x2f62a8),
            ty: rgb(0x11806c),
        },
        crew: [
            rgb(0x5d7a0e),
            rgb(0x11806c),
            rgb(0x6a4fc2),
            rgb(0xa23a9e),
            rgb(0xb8456e),
            rgb(0x2a72b5),
        ],
    },
    false,
);

pub const DARK: Palette = Palette::new(
    Base {
        bg: rgb(0x2c2a2e),
        fg: rgb(0xe6e2dc),
        ember: rgb(0xf39050),
        glow: rgb(0xff9b4a),
        coal: rgb(0xd8622e),
        amber: rgb(0xf2b84b),
        amber_glow: rgb(0xf2b84b),
        good: rgb(0x7fd38a),
        bad: rgb(0xf0706a),
        plan: rgb(0x8ab4f0),
        auto: rgb(0xb89cf5),
        code: rgb(0xe9c48f),
        syntax: Syntax {
            kw: rgb(0xd49cf0),
            string: rgb(0xb5d98a),
            num: rgb(0xf3a06a),
            function: rgb(0x8ab4f0),
            ty: rgb(0x6fd0bd),
        },
        crew: [
            rgb(0xc2d86a),
            rgb(0x5cc8b4),
            rgb(0xb39cf2),
            rgb(0xdc8cf0),
            rgb(0xf08cb8),
            rgb(0x7cb8f0),
        ],
    },
    true,
);

impl Palette {
    const fn new(base: Base, dark: bool) -> Self {
        let you = mix(base.bg, base.fg, if dark { 0.065 } else { 0.052 });
        Self {
            base,
            shades: Shades {
                muted: mix(base.fg, base.bg, if dark { 0.3 } else { 0.32 }),
                dim: mix(base.fg, base.bg, if dark { 0.52 } else { 0.55 }),
                faint: mix(base.fg, base.bg, if dark { 0.8 } else { 0.84 }),
                you,
                well: mix(base.bg, base.fg, if dark { 0.038 } else { 0.03 }),
                well_deep: mix(base.bg, base.fg, if dark { 0.085 } else { 0.07 }),
                pick: mix(you, base.glow, if dark { 0.2 } else { 0.17 }),
                pick_bg: mix(base.bg, base.glow, if dark { 0.16 } else { 0.13 }),
                add_tint: mix(base.bg, base.good, if dark { 0.14 } else { 0.12 }),
                del_tint: mix(base.bg, base.bad, if dark { 0.14 } else { 0.1 }),
                add_word: mix(base.bg, base.good, if dark { 0.3 } else { 0.26 }),
                del_word: mix(base.bg, base.bad, if dark { 0.3 } else { 0.22 }),
                bad_tint: mix(base.bg, base.bad, if dark { 0.12 } else { 0.08 }),
                amber_tint: mix(you, base.amber_glow, 0.12),
                glow_tint: mix(base.bg, base.glow, if dark { 0.12 } else { 0.1 }),
            },
        }
    }
    pub fn detected(
        self,
        foreground: Option<(u16, u16, u16)>,
        ground: (u16, u16, u16),
        light: bool,
    ) -> Self {
        let color = |(r, g, b): (u16, u16, u16)| {
            Color::Rgb((r / 257) as u8, (g / 257) as u8, (b / 257) as u8)
        };
        Self::new(
            Base {
                bg: color(ground),
                fg: foreground.map(color).unwrap_or(self.base.fg),
                ..self.base
            },
            !light,
        )
    }
    pub const fn bg(self) -> Color {
        self.base.bg
    }
    pub const fn fg(self) -> Color {
        self.base.fg
    }
    pub const fn ember(self) -> Color {
        self.base.ember
    }
    pub const fn glow(self) -> Color {
        self.base.glow
    }
    pub const fn coal(self) -> Color {
        self.base.coal
    }
    pub const fn amber(self) -> Color {
        self.base.amber
    }
    pub const fn amber_glow(self) -> Color {
        self.base.amber_glow
    }
    pub const fn good(self) -> Color {
        self.base.good
    }
    pub const fn bad(self) -> Color {
        self.base.bad
    }
    pub const fn plan(self) -> Color {
        self.base.plan
    }
    pub const fn auto(self) -> Color {
        self.base.auto
    }
    pub const fn code(self) -> Color {
        self.base.code
    }
    pub const fn syntax(self) -> Syntax {
        self.base.syntax
    }
    pub const fn crew(self) -> [Color; 6] {
        self.base.crew
    }
    pub const fn muted(self) -> Color {
        self.shades.muted
    }
    pub const fn dim(self) -> Color {
        self.shades.dim
    }
    pub const fn faint(self) -> Color {
        self.shades.faint
    }
    pub const fn you(self) -> Color {
        self.shades.you
    }
    pub const fn well(self) -> Color {
        self.shades.well
    }
    pub const fn well_deep(self) -> Color {
        self.shades.well_deep
    }
    pub const fn pick(self) -> Color {
        self.shades.pick
    }
    pub const fn pick_bg(self) -> Color {
        self.shades.pick_bg
    }
    pub const fn add_tint(self) -> Color {
        self.shades.add_tint
    }
    pub const fn del_tint(self) -> Color {
        self.shades.del_tint
    }
    pub const fn add_word(self) -> Color {
        self.shades.add_word
    }
    pub const fn del_word(self) -> Color {
        self.shades.del_word
    }
    pub const fn bad_tint(self) -> Color {
        self.shades.bad_tint
    }
    pub const fn amber_tint(self) -> Color {
        self.shades.amber_tint
    }
    pub const fn glow_tint(self) -> Color {
        self.shades.glow_tint
    }
    pub fn named(self, name: &str) -> Option<Color> {
        Some(match name {
            "bg" => self.bg(),
            "fg" => self.fg(),
            "ember" => self.ember(),
            "glow" => self.glow(),
            "coal" => self.coal(),
            "amber" => self.amber(),
            "amberGlow" => self.amber_glow(),
            "good" => self.good(),
            "bad" => self.bad(),
            "plan" => self.plan(),
            "auto" => self.auto(),
            "code" => self.code(),
            "kw" => self.syntax().kw,
            "str" => self.syntax().string,
            "num" => self.syntax().num,
            "fn" => self.syntax().function,
            "ty" => self.syntax().ty,
            "crew0" => self.crew()[0],
            "crew1" => self.crew()[1],
            "crew2" => self.crew()[2],
            "crew3" => self.crew()[3],
            "crew4" => self.crew()[4],
            "crew5" => self.crew()[5],
            "muted" => self.muted(),
            "dim" => self.dim(),
            "faint" => self.faint(),
            "you" => self.you(),
            "well" => self.well(),
            "wellDeep" => self.well_deep(),
            "pick" => self.pick(),
            "pickBg" => self.pick_bg(),
            "addTint" => self.add_tint(),
            "delTint" => self.del_tint(),
            "addWord" => self.add_word(),
            "delWord" => self.del_word(),
            "badTint" => self.bad_tint(),
            "amberTint" => self.amber_tint(),
            "glowTint" => self.glow_tint(),
            "ash" => self.muted(),
            "presence" | "attention" => self.ember(),
            _ => return parse_hex(name),
        })
    }
}

pub const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn parse_hex(name: &str) -> Option<Color> {
    let hex = name.strip_prefix('#').filter(|hex| hex.len() == 6)?;
    u32::from_str_radix(hex, 16).ok().map(rgb)
}

pub const fn mix(from: Color, to: Color, t: f64) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(x, y, z)) = (from, to) else {
        return to;
    };
    let t = if t < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        t
    };
    Color::Rgb(channel(r, x, t), channel(g, y, t), channel(b, z, t))
}
/// The coal under an ink that is not ember's: the ink a shade darker
/// (`coalOf`).
pub const fn shaded(color: Color) -> Color {
    mix(color, Color::Rgb(0, 0, 0), 0.28)
}

const fn channel(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t + 0.5) as u8
}

#[cfg(test)]
mod tests;

/// Resolve the terminal default before a colour is interpolated.
pub fn foreground(ink: Option<Color>, palette: Palette) -> Color {
    match ink {
        None | Some(Color::Reset) => palette.fg(),
        Some(ink) => ink,
    }
}
