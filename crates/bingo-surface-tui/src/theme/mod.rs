//! Ember tokens, depth and glyphs for every drawing pass.

use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use unicode_width::UnicodeWidthStr;
mod depth;
pub mod glyphs;
mod ground;
pub mod ink;
use depth::nearest;
pub use glyphs::{ASCII, Glyphs, UNICODE};
pub use ground::background_is_light;
pub(crate) use ground::{answered_foreground, answered_ground};
pub use ink::{DARK, LIGHT, Palette};

/// How much colour a terminal is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colors {
    /// `NO_COLOR`: weight and shape carry every fact colour would have.
    Plain,
    /// The eight every terminal is sure of.
    Ansi,
    /// 24-bit, the native look.
    True(Palette),
    Indexed(Palette),
}

/// Terminal bell, written out of band between frames.
pub const BELL: &[u8] = b"\x07";

// ---- the look, and the ground it may change under -----------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub colors: Colors,
    pub glyphs: &'static Glyphs,
}

/// Which of the two truecolor palettes a person wants: the one their terminal
/// is already wearing, or one they named.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    #[default]
    Terminal,
    Light,
    Dark,
}

/// `BINGO_THEME`. It sits with `BINGO_MOTION` and `BINGO_ASCII` rather than in
/// the settings file because the look is chosen before the kernel is up: a
/// setting would arrive after the first frame had already been drawn.
pub fn look(setting: Option<&str>) -> Look {
    match setting {
        Some("light") => Look::Light,
        Some("dark") => Look::Dark,
        _ => Look::Terminal,
    }
}

/// Everything the look is chosen from that no answer can change: what the
/// environment asks for, read once at start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ask {
    /// `NO_COLOR`, which wins over all of it.
    pub no_color: bool,
    /// `COLORTERM` says 24 bits are safe.
    pub truecolor: bool,
    pub indexed: bool,
    pub ascii: bool,
    pub look: Look,
}

/// What the environment asks for, over the ground the terminal last said it
/// has — `None` where it did not answer or was never asked. Truecolor is
/// announced by `COLORTERM`; a terminal that says nothing gets the eight it is
/// sure of. `NO_COLOR` wins over every other answer, the terminal's own
/// included.
pub fn choose(ask: Ask, light: Option<bool>) -> Theme {
    let palette = match (ask.look, light) {
        (Look::Light, _) | (Look::Terminal, Some(true)) => LIGHT,
        _ => DARK,
    };
    let colors = match (ask.no_color, ask.truecolor) {
        (true, _) => Colors::Plain,
        (false, true) => Colors::True(palette),
        (false, false) if ask.indexed => Colors::Indexed(palette),
        (false, false) => Colors::Ansi,
    };
    Theme {
        colors,
        glyphs: if ask.ascii { &ASCII } else { &UNICODE },
    }
}

/// The look of the active pass, or the resting look outside a pass.
pub fn current() -> Theme {
    crate::render::current().look
}

/// Pure detection, shared by the run and the fixtures.
#[cfg(test)]
pub fn detect(ask: Ask, ground: Option<(u16, u16, u16)>) -> Theme {
    detect_palette(ask, None, ground)
}

pub fn detect_palette(
    ask: Ask,
    foreground: Option<(u16, u16, u16)>,
    ground: Option<(u16, u16, u16)>,
) -> Theme {
    let light = ground.map(|(r, g, b)| background_is_light(r, g, b));
    let mut theme = choose(ask, light);
    if follows(ask)
        && let Some(ground) = ground
    {
        match &mut theme.colors {
            Colors::True(palette) | Colors::Indexed(palette) => {
                *palette = palette.detected(foreground, ground, light == Some(true))
            }
            _ => {}
        }
    }
    theme
}

#[cfg(test)]
pub(crate) fn reference_ground(light: bool) -> (u16, u16, u16) {
    let Color::Rgb(r, g, b) = (if light { LIGHT.bg() } else { DARK.bg() }) else {
        return (0, 0, 0);
    };
    (u16::from(r) * 257, u16::from(g) * 257, u16::from(b) * 257)
}

pub fn follows(ask: Ask) -> bool {
    !ask.no_color && (ask.truecolor || ask.indexed) && ask.look == Look::Terminal
}

/// How long a probe of the terminal is given, this one and the graphics probe
/// beside it ([`crate::graphics`]). A terminal that has not answered by then is
/// one whose answer would have arrived after the first frame.
pub(crate) const PROBE: std::time::Duration = std::time::Duration::from_millis(400);

/// How long the graphics probe is given when tmux is carrying the questions
/// and the answers, and tmux has said it carries them ([`crate::graphics`],
/// M60 brick 3).
///
/// Three legs where a bare terminal has one: the question crosses to tmux's
/// server, waits there for the flush that hands it to the client's terminal,
/// and the answer comes back the same way through the server into this pane —
/// each leg waiting on tmux's own event loop rather than on the terminal's.
/// [`PROBE`] is one leg's worth, and the box this was reported from (tmux
/// 3.6b under Ghostty) took longer than one; three is the same clock read
/// three times rather than a second number to keep in step.
///
/// It is public because the pty harness's late scene has to outlast it, and a
/// window the test spells for itself is a window that drifts.
pub const PROBE_THROUGH: std::time::Duration =
    std::time::Duration::from_millis(3 * PROBE.as_millis() as u64);

// ---- and asked again (M71) ----------------------------------------------

/// The question, asked again while the run lasts at the moments a change of
/// palette is likely to change ([`crate::run::look`]): OSC 10 and OSC 11,
/// the foreground and background queried at startup too. The answer comes back through
/// crossterm's key stream, where [`crate::late`] hears it whole and
/// [`answered_ground`] reads it.
pub const QUESTION: &[u8] = b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\";

/// Draw whatever `f` draws in another look.
#[cfg(test)]
pub fn with<R>(theme: Theme, f: impl FnOnce() -> R) -> R {
    crate::render::Render {
        look: theme,
        ..crate::render::current()
    }
    .scope(f)
}

// ---- the tokens ---------------------------------------------------------

/// Identity colours belong only to agent sigils and names.
pub fn crew(index: usize) -> Style {
    match current().colors {
        Colors::True(palette) => Style::new().fg(palette.crew()[index % 6]),
        Colors::Indexed(palette) => Style::new().fg(nearest(palette.crew()[index % 6])),
        _ => text(),
    }
}

/// Answers, what you type, option labels — the terminal's own foreground, in
/// every look (M73). No palette names it: a terminal that flips its scheme
/// remaps every cell of prose itself, on the spot and in its scrollback, which
/// is why Codex and Claude Code both draw their prose with no colour at all.
/// It is spelled `Reset` rather than left unsaid so that it can be patched
/// over a colour — the end of a ramp, a bold name a light has left.
pub fn text() -> Style {
    Style::new().fg(Color::Reset)
}

/// Secondary text blends the terminal's foreground and ground; without
/// colour, its weight carries the distinction.
pub fn dim() -> Style {
    foreground("dim").unwrap_or_default()
}

/// The bar behind a `>` line, a card's surface, a placed view. Never text.
pub fn raised() -> Style {
    match current().colors {
        Colors::True(palette) => Style::new().bg(palette.you()),
        Colors::Indexed(palette) => Style::new().bg(nearest(palette.you())),
        _ => Style::new(),
    }
}

/// bingo's own colour: the sparkle, a running step, a card's border, the focused
/// row. The one warm colour, and the only one that moves.
pub fn presence() -> Style {
    foreground("ember").unwrap_or_default()
}

/// The bright half of a pulse: everything that breathes moves between this
/// and [`presence`]. Views reach it through [`pulse`] and [`comet`] rather
/// than directly, because what moves is a phase and not a colour.
pub fn glow() -> Style {
    foreground("glow").unwrap_or_default()
}

pub fn good() -> Style {
    foreground("good").unwrap_or_default()
}

pub fn bad() -> Style {
    foreground("bad").unwrap_or_default()
}

/// The `⏵⏵` on the status line, and links — the one cool colour.
pub fn mode() -> Style {
    foreground("plan").unwrap_or_default()
}

/// A diff's rows: `good` and `bad` on their own tints, which only 24 bits can
/// draw; the column's `+` and `-` carry it everywhere else.
pub fn added() -> Style {
    tinted(good(), |p| p.add_tint())
}

pub fn removed() -> Style {
    tinted(bad(), |p| p.del_tint())
}

fn tinted(fg: Style, tint: fn(Palette) -> Color) -> Style {
    match current().colors {
        Colors::True(palette) => fg.bg(tint(palette)),
        Colors::Indexed(palette) => fg.bg(nearest(tint(palette))),
        _ => fg,
    }
}

#[cfg(test)]
pub fn rule_style() -> Style {
    foreground("faint").unwrap_or_default()
}
pub fn ash() -> Style {
    foreground("muted").unwrap_or_default()
}

// ---- what moves (design §6) ---------------------------------------------
//
// Every one of these takes a phase — a number between 0 and 1 that
// [`crate::clock`] derived from the clock — and answers with the token at that
// point. The colour stays here; the timing stays there; a view names neither.

/// The steps a breath is drawn in. Twenty-four bits can show five; the eight
/// have only the two ends, and a terminal without colour has neither.
pub const BREATH_STEPS: u8 = 5;

/// How dim a breath goes: 65 % of `presence` (§6).
const BREATH_FLOOR: f32 = 0.65;

/// What wants a person pulses once a second (§6).
pub const PULSE: std::time::Duration = std::time::Duration::from_secs(1);

/// bingo breathing — the activity row's sparkle and the input box's border
/// while a turn runs. `level` is 0 at the bottom of the breath and 1 at the
/// top of it.
pub fn breath(level: f32) -> Style {
    let level = steps(level, BREATH_STEPS);
    match current().colors {
        Colors::Plain => Style::new(),
        Colors::Ansi | Colors::Indexed(_) => two_ways(level, dim(), presence()),
        Colors::True(palette) => Style::new().fg(scaled(
            palette.ember(),
            BREATH_FLOOR + (1.0 - BREATH_FLOOR) * level,
        )),
    }
}

/// A live mark pulsing between `presence` and its glow.
pub fn pulse(level: f32) -> Style {
    match current().colors {
        Colors::Plain => Style::new(),
        Colors::Ansi | Colors::Indexed(_) => two_ways(level, presence(), glow()),
        Colors::True(palette) => {
            Style::new().fg(ink::mix(palette.ember(), palette.glow(), f64::from(level)))
        }
    }
}

/// The comet tail on streaming text: the cell that just arrived wears the
/// glow, the cells behind it cool through `presence`, and the tail beyond
/// [`SETTLES`] is the resting ink. One of the two gradients §4 sanctions, and
/// the one place a ramp is still drawn towards the ink — which the terminal
/// owns since M73, so the ink is where the ramp *ends* rather than a colour it
/// can pass through.
pub fn comet(age: f32) -> Style {
    let age = settled(age);
    match current().colors {
        Colors::Plain => Style::new(),
        Colors::Ansi | Colors::Indexed(_) => two_ways(age, glow(), text()),
        Colors::True(palette) if age < SETTLES => Style::new().fg(ink::mix(
            palette.glow(),
            palette.ember(),
            f64::from(age / SETTLES),
        )),
        Colors::True(_) => text(),
    }
}

/// What wants a person, wherever it is said — the `N needs you` notice, a
/// waiting child's row, its line in the switcher. It alternates with plain
/// text once a second so the eye is drawn back to it, and rests on bingo's
/// own colour where nothing may move (§6).
pub fn attention(now: crate::clock::Now) -> Style {
    match now.motion && crate::clock::alternating(now, PULSE) {
        true => text(),
        false => presence(),
    }
}

/// A notice arriving and leaving: the words' own weight at both edges of its
/// life, its own level's colour while it is there to be read. `dim` is a
/// weight and not a hue since M73, so there is nothing left to mix through —
/// the notice is dim until it has arrived, as it has always been on the eight.
#[cfg(test)]
pub fn fading(level: bingo_sdk::Level, t: f32) -> Style {
    match t >= 1.0 {
        true => self::level(level),
        false => dim(),
    }
}

/// The light that crosses a tool's name as its answer lands: `good` at the
/// crest and the row's own weight where it has passed. The bullet is what
/// says the call finished; this says only how fresh that is (§6), so a name
/// the light has left carries no colour of its own — and that is the
/// terminal's ink now (M73), which no ramp reaches, so the light is the two
/// stops the eight colours always drew it in.
pub fn landing(level: f32) -> Style {
    if level <= 0.0 {
        return bold();
    }
    match current().colors {
        Colors::Plain => bold(),
        _ => two_ways(level, bold(), good().patch(bold())),
    }
}

/// A failure cooling into the words behind it: `bad` where it lands and the
/// resting ink once it has settled. The bullet stays `bad`, so what cools is
/// how fresh the failure is and never whether there was one (§4).
pub fn cooling(t: f32) -> Style {
    match current().colors {
        Colors::Plain => Style::new(),
        _ => two_ways(t, bad(), text()),
    }
}

/// The context notice warming from the words' own weight towards `bad` as the
/// window fills. It starts where secondary text does, which is a weight and
/// no colour at all (M73), so the notice takes `bad` at [`SETTLES`] rather
/// than creeping towards it through a grey.
#[cfg(test)]
pub fn warming(t: f32) -> Style {
    match current().colors {
        Colors::Plain => Style::new(),
        _ => two_ways(t, dim(), bad()),
    }
}

/// A share of something, held to 0 and 1 — and to 0 where the arithmetic it
/// came out of had nothing to divide, so no `NaN` ever reaches a colour.
fn settled(share: f32) -> f32 {
    match share.is_nan() {
        true => 0.0,
        false => share.clamp(0.0, 1.0),
    }
}

/// Where a ramp hands over to its other end: the halfway mark, for the eight
/// colours that cannot draw a ramp at all and for the ramps 24 bits cannot
/// finish either, because the ink at the end of them is the terminal's (M73).
const SETTLES: f32 = 0.5;

/// A ramp the eight colours cannot draw collapses to its two ends, and
/// [`SETTLES`] is where it changes over.
fn two_ways(level: f32, low: Style, high: Style) -> Style {
    match level >= SETTLES {
        true => high,
        false => low,
    }
}

/// A continuous level, snapped to the steps a ramp is drawn in.
fn steps(level: f32, steps: u8) -> f32 {
    let last = f32::from(steps.saturating_sub(1)).max(1.0);
    (level.clamp(0.0, 1.0) * last).round() / last
}

/// A colour at a share of its own brightness — how a breath dims without
/// leaving the hue it belongs to.
fn scaled(colour: Color, share: f32) -> Color {
    let Color::Rgb(r, g, b) = colour else {
        return colour;
    };
    let share = share.clamp(0.0, 1.0);
    let at = |c: u8| (f32::from(c) * share).round() as u8;
    Color::Rgb(at(r), at(g), at(b))
}

// ---- syntax inks -------------------------------------------------------

/// What a highlighter may say about a run of code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Keyword,
    String,
    Number,
    Function,
    Type,
    Comment,
    Plain,
}

/// The TextMate scope prefixes each ink answers to — the vocabulary every
/// syntax in the set is written in — tried in this order. A scope that matches
/// none of them is [`Ink::Plain`]; a syntax this table has never heard of
/// still reads as code, which is why the fallback is the answer and not a
/// colour of its own.
pub const INKS: &[(Ink, &str)] = &[
    (Ink::Comment, "comment"),
    (Ink::String, "string"),
    (Ink::Number, "constant.numeric"),
    (Ink::Function, "entity.name.function"),
    (Ink::Function, "support.function"),
    (Ink::Type, "entity.name.type"),
    (Ink::Type, "support.type"),
    (Ink::Type, "support.class"),
    // An operator is punctuation, not vocabulary: colouring `=` and `&&`
    // would put the cool colour on every other cell of a line.
    (Ink::Plain, "keyword.operator"),
    (Ink::Keyword, "keyword"),
    (Ink::Keyword, "storage"),
    (Ink::Keyword, "constant.language"),
    (Ink::Keyword, "variable.language"),
    (Ink::Keyword, "entity.name.tag"),
];

/// One ink as a token of §4's table.
pub fn ink(ink: Ink) -> Style {
    match ink {
        Ink::Keyword => foreground("kw").unwrap_or_default(),
        Ink::String => foreground("str").unwrap_or_default(),
        Ink::Number => foreground("num").unwrap_or_default(),
        Ink::Function => foreground("fn").unwrap_or_default(),
        Ink::Type => foreground("ty").unwrap_or_default(),
        Ink::Comment => dim().patch(italic()),
        Ink::Plain => text(),
    }
}

pub fn bold() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

pub fn italic() -> Style {
    Style::new().add_modifier(Modifier::ITALIC)
}

pub fn struck() -> Style {
    Style::new().add_modifier(Modifier::CROSSED_OUT)
}

pub fn link() -> Style {
    mode().add_modifier(Modifier::UNDERLINED)
}

/// Every foreground §4's table can put in a cell, in the look now standing:
/// the accents, and each ramp between them sampled finely enough that any
/// point one of them reaches is in the set. It is derived from the tokens
/// themselves rather than from the palette, so a ramp added to the table is in
/// it without anybody remembering to say so.
///
/// What a drawn screen carries outside this set, `Reset` aside — and `Reset`
/// is the ink and the dim both since M73 — is a colour nobody sanctioned.
/// `crate::screens` holds every screen to it.
#[cfg(test)]
pub fn spendable() -> std::collections::HashSet<Color> {
    let mut out: std::collections::HashSet<Color> = std::collections::HashSet::new();
    let mut keep = |style: Style| {
        if let Some(colour) = style.fg {
            out.insert(colour);
        }
    };
    for style in [
        presence(),
        glow(),
        good(),
        bad(),
        mode(),
        rule_style(),
        ash(),
    ] {
        keep(style);
    }
    for index in 0..6 {
        keep(crew(index));
    }
    for step in 0..=SAMPLES {
        let t = step as f32 / SAMPLES as f32;
        for style in [
            breath(t),
            pulse(t),
            comet(t),
            landing(t),
            cooling(t),
            warming(t),
        ] {
            keep(style);
        }
        for level in [
            bingo_sdk::Level::Info,
            bingo_sdk::Level::Warn,
            bingo_sdk::Level::Error,
        ] {
            keep(fading(level, t));
        }
    }
    out
}

/// How finely [`spendable`] samples a ramp. A ramp's channels move by at most
/// 255 over its whole length, so a thousand steps reach every colour any `t`
/// can land on.
#[cfg(test)]
const SAMPLES: u32 = 1_000;

/// A token as a drawn cell wears it: a terminal gives every cell a colour, so
/// an unpainted one reads back as `Reset` rather than as nothing. Comparing a
/// buffer's cell to a token goes through here.
#[cfg(test)]
pub fn as_drawn(style: Style) -> Style {
    let mut cell = ratatui::buffer::Cell::default();
    cell.set_style(style);
    cell.style()
}

pub fn level(level: bingo_sdk::Level) -> Style {
    match level {
        bingo_sdk::Level::Info => dim(),
        bingo_sdk::Level::Warn => presence(),
        bingo_sdk::Level::Error => bad(),
    }
}

// ---- the glyph table ----------------------------------------------------

pub fn glyphs() -> &'static Glyphs {
    current().glyphs
}

/// What the model says and does: `▍` at column 1, its text at 3.
pub fn bullet() -> &'static str {
    glyphs().bullet
}

/// A producer's one-cell mark follows the terminal's glyph capabilities.
pub fn row_mark(mark: char) -> String {
    use unicode_width::UnicodeWidthChar;
    if mark.width() != Some(1) || (bullet().is_ascii() && !mark.is_ascii()) {
        bullet().to_string()
    } else {
        mark.to_string()
    }
}

/// What came back: `⎿` at column 2, its text at 5.
pub fn connector() -> &'static str {
    glyphs().connector
}

pub fn user() -> &'static str {
    glyphs().user
}

pub fn cursor() -> &'static str {
    glyphs().cursor
}

/// Where typing lands on a row the terminal's own cursor is not parked on.
pub fn caret() -> &'static str {
    glyphs().caret
}

/// `❯` marks what the keyboard talks to (design §7), and the rows it does not
/// talk to keep its width so nothing shifts when the focus moves.
pub fn cursor_span(focused: bool) -> ratatui::text::Span<'static> {
    match focused {
        true => ratatui::text::Span::styled(format!("{} ", cursor()), presence()),
        false => ratatui::text::Span::raw(" ".repeat(cursor().width() + 1)),
    }
}

/// The three marks a task on the list can wear, still to do first
/// ([`Glyphs::task`]); which one a task gets is [`crate::tasks`]'s to say.
pub fn tasks() -> [&'static str; 3] {
    glyphs().task
}

pub fn todo(done: bool) -> &'static str {
    glyphs().todo[usize::from(done)]
}

/// The tick a card spends where no box is drawn round it: `✔ Submit` on a
/// form's tab row, and the mark inside a multi-select's `[✔]`.
pub fn tick() -> &'static str {
    glyphs().tick
}

/// A rule between blocks: the same stroke a box draws its edge with, because
/// they are one line and not two facts.
pub fn rule() -> &'static str {
    glyphs().border.horizontal_top
}

/// A tree node that has siblings under it; [`corner`] is the last of them.
pub fn branch() -> &'static str {
    glyphs().tree[0]
}

pub fn corner() -> &'static str {
    glyphs().tree[1]
}

/// What is left where something was folded away, cut short, or is still on
/// its way: a fold's `… +12 lines`, a middle-elided path, `Thinking…`.
pub fn ellipsis() -> &'static str {
    glyphs().ellipsis
}

/// What opens an item of a list, and the wall a quotation hangs from — the
/// border's own upright, so a box and a quote agree.
pub fn point() -> &'static str {
    glyphs().point
}

pub fn wall() -> &'static str {
    glyphs().border.vertical_left
}

pub fn border() -> border::Set<'static> {
    glyphs().border
}

/// bingo's voice, at work or at rest.
pub fn mark() -> &'static str {
    glyphs().mark[0]
}

/// The narrower caret while a thought is being had.
pub fn thinking() -> &'static str {
    glyphs().mark[1]
}

#[cfg(test)]
mod tests;

pub fn palette() -> Palette {
    match current().colors {
        Colors::True(palette) | Colors::Indexed(palette) => palette,
        _ => DARK,
    }
}

pub fn foreground(name: &str) -> Option<Style> {
    if name == "fg" {
        return Some(text());
    }
    palette()
        .named(name)
        .map(|color| depth::foreground(name, color))
        .or_else(|| name.parse().ok().map(depth::literal))
}

pub fn ground(name: &str) -> Option<Style> {
    palette()
        .named(name)
        .map(|color| depth::ground(name, color))
}

#[allow(dead_code)]
pub fn blended(from: &str, to: &str, weight: f64) -> Option<Style> {
    let palette = palette();
    let color = ink::mix(palette.named(from)?, palette.named(to)?, weight);
    Some(depth::foreground(from, color))
}

// The composer adopts this fade when its Ember layout lands.
#[allow(dead_code)]
pub fn caret_ink(now: crate::clock::Now, base: &str, ground: &str) -> Option<Style> {
    blended(base, ground, crate::clock::caret_fade(now))
}

pub(crate) use depth::literal;
