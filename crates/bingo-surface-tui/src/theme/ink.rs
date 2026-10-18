//! The prototype palette, with rounding at each sRGB blend.
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub ember: Color,
    pub glow: Color,
    pub coal: Color,
    pub amber: Color,
    pub amber_glow: Color,
    pub good: Color,
    pub bad: Color,
    pub plan: Color,
    pub auto: Color,
    pub code: Color,
    pub syntax: Syntax,
    pub crew: [Color; 6],
    pub shades: Shades,
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
pub struct Shades {
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

pub const LIGHT: Palette = Palette {
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
    shades: Shades {
        muted: Color::Reset,
        dim: Color::Reset,
        faint: Color::Reset,
        you: Color::Reset,
        well: Color::Reset,
        well_deep: Color::Reset,
        pick: Color::Reset,
        pick_bg: Color::Reset,
        add_tint: Color::Reset,
        del_tint: Color::Reset,
        add_word: Color::Reset,
        del_word: Color::Reset,
        bad_tint: Color::Reset,
        amber_tint: Color::Reset,
        glow_tint: Color::Reset,
    },
}
.derive(false);

pub const DARK: Palette = Palette {
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
    shades: Shades {
        muted: Color::Reset,
        dim: Color::Reset,
        faint: Color::Reset,
        you: Color::Reset,
        well: Color::Reset,
        well_deep: Color::Reset,
        pick: Color::Reset,
        pick_bg: Color::Reset,
        add_tint: Color::Reset,
        del_tint: Color::Reset,
        add_word: Color::Reset,
        del_word: Color::Reset,
        bad_tint: Color::Reset,
        amber_tint: Color::Reset,
        glow_tint: Color::Reset,
    },
}
.derive(true);

impl Palette {
    const fn derive(mut self, dark: bool) -> Self {
        self.shades.muted = mix(self.fg, self.bg, if dark { 0.3 } else { 0.32 });
        self.shades.dim = mix(self.fg, self.bg, if dark { 0.52 } else { 0.55 });
        self.shades.faint = mix(self.fg, self.bg, if dark { 0.8 } else { 0.84 });
        self.shades.you = mix(self.bg, self.fg, if dark { 0.065 } else { 0.052 });
        self.shades.well = mix(self.bg, self.fg, if dark { 0.038 } else { 0.03 });
        self.shades.well_deep = mix(self.bg, self.fg, if dark { 0.085 } else { 0.07 });
        self.shades.pick = mix(self.shades.you, self.glow, if dark { 0.2 } else { 0.17 });
        self.shades.pick_bg = mix(self.bg, self.glow, if dark { 0.16 } else { 0.13 });
        self.shades.add_tint = mix(self.bg, self.good, if dark { 0.14 } else { 0.12 });
        self.shades.del_tint = mix(self.bg, self.bad, if dark { 0.14 } else { 0.1 });
        self.shades.add_word = mix(self.bg, self.good, if dark { 0.3 } else { 0.26 });
        self.shades.del_word = mix(self.bg, self.bad, if dark { 0.3 } else { 0.22 });
        self.shades.bad_tint = mix(self.bg, self.bad, if dark { 0.12 } else { 0.08 });
        self.shades.amber_tint = mix(self.shades.you, self.amber_glow, 0.12);
        self.shades.glow_tint = mix(self.bg, self.glow, if dark { 0.12 } else { 0.1 });
        self
    }
    pub fn detected(
        mut self,
        foreground: Option<(u16, u16, u16)>,
        ground: (u16, u16, u16),
        light: bool,
    ) -> Self {
        let color = |(r, g, b): (u16, u16, u16)| {
            Color::Rgb((r / 257) as u8, (g / 257) as u8, (b / 257) as u8)
        };
        self.bg = color(ground);
        if let Some(foreground) = foreground {
            self.fg = color(foreground);
        }
        self.derive(!light)
    }
    pub fn named(self, name: &str) -> Option<Color> {
        Some(match name {
            "bg" => self.bg,
            "fg" => self.fg,
            "ember" => self.ember,
            "glow" => self.glow,
            "coal" => self.coal,
            "amber" => self.amber,
            "amberGlow" => self.amber_glow,
            "good" => self.good,
            "bad" => self.bad,
            "plan" => self.plan,
            "auto" => self.auto,
            "code" => self.code,
            "kw" => self.syntax.kw,
            "str" => self.syntax.string,
            "num" => self.syntax.num,
            "fn" => self.syntax.function,
            "ty" => self.syntax.ty,
            "crew0" => self.crew[0],
            "crew1" => self.crew[1],
            "crew2" => self.crew[2],
            "crew3" => self.crew[3],
            "crew4" => self.crew[4],
            "crew5" => self.crew[5],
            "muted" => self.shades.muted,
            "dim" => self.shades.dim,
            "faint" => self.shades.faint,
            "you" => self.shades.you,
            "well" => self.shades.well,
            "wellDeep" => self.shades.well_deep,
            "pick" => self.shades.pick,
            "pickBg" => self.shades.pick_bg,
            "addTint" => self.shades.add_tint,
            "delTint" => self.shades.del_tint,
            "addWord" => self.shades.add_word,
            "delWord" => self.shades.del_word,
            "badTint" => self.shades.bad_tint,
            "amberTint" => self.shades.amber_tint,
            "glowTint" => self.shades.glow_tint,
            "ash" => self.shades.muted,
            "presence" => self.ember,
            _ => return parse_hex(name),
        })
    }
}

impl std::ops::Deref for Palette {
    type Target = Shades;
    fn deref(&self) -> &Self::Target {
        &self.shades
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
const fn channel(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t + 0.5) as u8
}

#[cfg(test)]
mod tests;
