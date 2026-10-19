//! Items draw as prompt bands, persistent step rows and answers (ADR-0097).
//! The reducer owns history; blocks memoise these projections of it.

use std::time::{Duration, Instant};

use bingo_sdk::{DecisionKind, Item, ItemBody, ItemStatus, SessionState, ToolOutput};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::clock::{self, Anim, Now};
use crate::fold::{self, Fold, Folds};
use crate::graphics::{Decoded, Linked, Picture};
use crate::tree::{self, Agents};
use crate::{markdown, paths, theme, thoughts, wrap};

mod answer;
mod history;
pub use history::History;
mod components;
/// What was said into a session: a person's line, a subsystem's notice, a
/// room's conversation.
mod said;
use components::item_lines;

pub(crate) mod band;
mod chips;
mod notice;
use crate::component::live;
/// Producer replacements for individual transcript rows.
pub(crate) mod slots;
pub mod sparks;

/// What is under a tool row: its output, folded to what a row can spare.
mod output;
/// The picture under the words, where the terminal draws one (§5).
mod pictured;
/// A shell line the person ran themselves (`!<command>`).
pub(crate) mod ran;
/// A turn's steps and narration at column 3.
mod steps;
pub(crate) use steps::asked;
pub mod opened;
/// The rows a thought draws, one of them or a run of them (§4, §6).
mod thinking;

pub use output::whole;
use output::{kept, plain, tail};
pub use ran::whole as shell_whole;
pub use thinking::thought;

pub(crate) use said::voice;

/// How long one light takes to cross the name of a call that has just come
/// back: six frames (§6). A thirty-three millisecond flash is below the
/// threshold at which a person reads it as something happening.
pub const SWEEP: Duration = Duration::from_millis(6 * crate::clock::FRAME.as_millis() as u64);
/// How long a call that came back wrong takes to cool out of `bad` into the
/// words behind it: twelve frames (§6). A flare that settles, never a shake —
/// §3's "nothing jumps" outranks it.
pub const FLARE: Duration = Duration::from_millis(12 * crate::clock::FRAME.as_millis() as u64);
/// How long a block that has just landed goes on being drawn again: the
/// longer of the two cues one can wear, so neither is cut short.
pub const LANDING: Duration = match SWEEP.as_millis() > FLARE.as_millis() {
    true => SWEEP,
    false => FLARE,
};
/// One pulse of a live tool's bullet (§6).
const PULSE: Duration = Duration::from_millis(1200);

/// Output rows kept under a finished tool row before the rest folds away.
const OUTPUT_ROWS: usize = 5;
/// A thought's rows, streaming or peeked at. Fewer than a tool's tail, and the
/// reason is the difference between the two: a tool's output is *read* — a
/// person is looking for the line that says what happened — and a thought is
/// read *past*. Two rows are enough to see one moving and to recognise where
/// it went; a third would spend a row of the transcript on working nobody
/// asked for.
const THOUGHT_ROWS: usize = 2;
/// What every row of one transcript needs to know about where it is.
#[derive(Clone)]
pub struct Rows<'a> {
    pub tools: crate::tool_rows::Looks,
    pub history: std::rc::Rc<History<'a>>,
    pub pulled: Option<&'a crate::pulled::Memo>,
    pub state: &'a SessionState,
    /// Whose ink each teammate wears and the names a mention can match,
    /// read off the tree this frame.
    pub faces: crate::face::Faces,
    pub width: usize,
    /// How much of each block a person opened or shut; everything else wears
    /// its kind's own start ([`crate::fold`]).
    pub opened: opened::Opened<'a>,
    /// The frame being drawn: what every cue below is a function of.
    pub now: Now,
    pub stop_asked: Option<&'a bingo_sdk::TurnId>,
    /// The pictures already turned into pixels. A row that draws one has to
    /// know how many pixels it is before it can say how many cells it takes,
    /// and that answer is kept rather than worked out again every frame.
    pub pictures: &'a Decoded,
    /// The pictures the words themselves named, as far as they have been read
    /// in (M51). A row asks and draws what is there; it never goes looking,
    /// which is the run's job between frames.
    pub linked: &'a Linked,
    /// Plugin placements read from this frame's folded session.
    pub slots: Vec<bingo_sdk::slot::Placed>,
    /// Each mode's label, derived once per frame from slots' Mode placement
    /// so a step's why never searches the placements.
    pub(crate) mode_labels: std::collections::BTreeMap<String, String>,
    /// A newer release than this build, when the start-up check found one
    /// (M63). Terminal-side, like the fold and the scroll: it is a fact about
    /// this machine, not about the conversation.
    pub update: Option<&'a str>,
    /// A person's region cap (`tui.measure`). Prose inside it keeps the
    /// 100-cell measure from column 3 (ADR-0097 §3).
    pub measure: Option<usize>,
    pub entrance: crate::welcome::Entrance<'a>,
}

impl<'a> std::ops::Deref for Rows<'a> {
    type Target = History<'a>;
    fn deref(&self) -> &Self::Target {
        &self.history
    }
}

fn mode_labels(slots: &[bingo_sdk::slot::Placed]) -> std::collections::BTreeMap<String, String> {
    let modes = slots.iter().find_map(|placed| match &placed.slot {
        bingo_sdk::slot::Slot::Mode { modes, .. } => Some(modes),
        _ => None,
    });
    modes
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let value = item.action.args.as_str()?;
            Some((
                value.to_owned(),
                bingo_sdk::slot::mode_label(value, modes?).to_owned(),
            ))
        })
        .collect()
}

impl<'a> Rows<'a> {
    /// One frame's worth of where a row is: the session in view names itself,
    /// the surface supplies the room it has, what a person opened, the
    /// catalogue and the clock.
    pub fn of(
        state: &'a SessionState,
        width: usize,
        folds: &'a Folds,
        pictures: &'a Decoded,
        linked: &'a Linked,
        now: Now,
    ) -> Self {
        Self::using(
            (state, History::of(state)),
            width,
            folds,
            pictures,
            linked,
            now,
        )
    }

    fn using(
        history: (&'a SessionState, std::rc::Rc<History<'a>>),
        width: usize,
        folds: &'a Folds,
        pictures: &'a Decoded,
        linked: &'a Linked,
        now: Now,
    ) -> Self {
        let (state, history) = history;
        let slots = bingo_sdk::slot::placed(state);
        let mode_labels = mode_labels(&slots);
        Self {
            history,
            tools: Default::default(),
            pulled: None,
            state,
            faces: Default::default(),
            width,
            opened: opened::Opened::new(folds),
            now,
            stop_asked: None,
            pictures,
            linked,
            slots,
            mode_labels,
            update: None,
            measure: None,
            entrance: crate::welcome::Entrance::default(),
        }
    }

    pub fn arriving(&self, item: &Item) -> bool {
        item.status == ItemStatus::Running
            && self
                .state
                .turn
                .as_ref()
                .is_some_and(|turn| item.turn.as_ref() == Some(&turn.id))
            && !self
                .state
                .interactions
                .iter()
                .any(|ask| ask.turn == item.turn)
    }

    pub fn turn_items(&self, item: &Item) -> &[&'a Item] {
        self.turns
            .get(&item.turn)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn components(mut self, memo: &'a crate::pulled::Memo) -> Self {
        self.pulled = Some(memo);
        self
    }

    /// What the welcome box has to add: the release this build could become.
    pub fn in_tree(mut self, tree: &tree::Tree) -> Self {
        self.faces = crate::face::Faces::of(tree);
        self
    }

    pub fn saying(self, update: Option<&'a str>) -> Self {
        Self { update, ..self }
    }

    /// The measure a person set for prose, as this run was told it.
    pub fn measuring(self, measure: Option<usize>) -> Self {
        Self { measure, ..self }
    }
}

/// Where one block is in its own motion (§6): the clock [`crate::blocks`]
/// measured for it, and whether it flipped into being finished — which is
/// what the light across its name, and the cooling of one that failed, are
/// both measured from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    pub since: Instant,
    pub flip: bool,
}

/// What the words of a row that has just landed are doing: one light crossing
/// the name of a call that came back, the whole row cooling out of `bad` for
/// one that came back wrong, or nothing at all — which is what every frame
/// but the twelve after it draws.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Landing {
    Settled,
    Landed(f32),
    Failed(f32),
}

impl Landing {
    /// How far into its own motion this block is. A block that did not just
    /// finish, and a surface where nothing may move, are both settled.
    fn of(cue: Cue, failed: bool, rows: &Rows<'_>) -> Self {
        if !cue.flip || !rows.now.motion {
            return Landing::Settled;
        }
        let len = match failed {
            true => FLARE,
            false => SWEEP,
        };
        let come = Anim::new(cue.since, len).progress(rows.now.instant);
        match failed {
            true => Landing::Failed(come),
            false => Landing::Landed(come),
        }
    }

    /// What the words after the name are drawn in.
    fn about(self) -> Style {
        match self {
            Landing::Failed(come) => theme::cooling(come),
            _ => theme::text(),
        }
    }
}

impl Rows<'_> {
    /// The cells this block is laid out in: the region's own width, or the
    /// narrower line a person asked for (design §7).
    fn measured(&self) -> usize {
        wrap::measure(self.width, self.measure)
    }

    /// The cells a result has, once the `⎿` gutter has taken its own.
    fn result_width(&self) -> usize {
        self.measured().saturating_sub(connector().width()).max(1)
    }

    /// A path as a person reads it, from this session's own directory.
    fn shorten(&self, text: &str) -> String {
        paths::shorten_in(text, &self.state.summary.cwd, paths::home())
    }
}

/// A receipt is the answer to the row above it, so it opens no block of its
/// own (design §4: the receipt joins the result).
pub fn joins_the_row_above(item: &Item) -> bool {
    matches!(item.body, ItemBody::PermissionReceipt { .. })
}

/// One item's block: the lines it draws, and the pictures those lines stand
/// for. The two are one rendering — a placeholder cell says nothing without
/// the picture behind it — so they are answered together and kept together
/// ([`crate::blocks`]), and a frame that draws no picture carries none.
#[derive(Debug, Default)]
pub struct Block {
    pub next: Option<Instant>,
    pub lines: Vec<Line<'static>>,
    pub pictures: Vec<Picture>,
    /// The destinations this block's own words named (M51), whatever became
    /// of them. It rides with the lines for the same reason the pictures do:
    /// a block is drawn once and cloned ever after, and the run reads this
    /// off the frame to go and fetch what the session has not got yet.
    pub wanted: Vec<String>,
}

impl From<crate::views::Drawing> for Block {
    fn from(drawing: crate::views::Drawing) -> Self {
        Self {
            lines: drawing.lines,
            next: drawing.next,
            ..Self::default()
        }
    }
}
impl Block {
    /// Discard picture placements and pending image destinations: component
    /// substitutions replace text only, while their callers retain those fields
    /// on the original block for the graphics pass.
    pub fn into_text_drawing(self) -> crate::views::Drawing {
        crate::views::Drawing {
            lines: self.lines,
            next: self.next,
        }
    }
}

impl From<Vec<Line<'static>>> for Block {
    /// Lines that named no picture, which is most of them.
    fn from(lines: Vec<Line<'static>>) -> Self {
        Block {
            lines,
            ..Block::default()
        }
    }
}

/// Where one item sits among the ones around it: everything a block needs of
/// its neighbours, asked once per item and handed down as one thing.
#[derive(Debug, Default)]
pub struct Place<'a> {
    /// The item before it, which a receipt answers.
    pub previous: Option<&'a Item>,
    /// The thoughts adjacent before it ([`crate::thoughts`]), which the last
    /// of a run draws as its own. Empty for everything that is not a thought.
    pub run: Vec<&'a Item>,
    /// Whether a newer thought has joined this one: then the run's block hangs
    /// on that one and this draws nothing at all.
    pub joined: bool,
    /// The part it plays in its turn ([`crate::turns`]).
    pub role: crate::turns::Role,
}

impl<'a> Place<'a> {
    /// Where the item at `at` sits.
    pub fn of<T: std::borrow::Borrow<Item>>(items: &'a [T], at: usize) -> Self {
        let thought = items
            .get(at)
            .is_some_and(|item| thoughts::is_thought(item.borrow()));
        Self {
            previous: at
                .checked_sub(1)
                .and_then(|before| items.get(before))
                .map(std::borrow::Borrow::borrow),
            run: match thought {
                true => thoughts::run_before(items, at)
                    .iter()
                    .map(std::borrow::Borrow::borrow)
                    .collect(),
                false => Vec::new(),
            },
            joined: thought && thoughts::next_is_thought(items, at),
            role: crate::turns::Role::Own,
        }
    }

    /// The same place, with the part the item plays in its turn.
    pub fn playing(self, role: crate::turns::Role) -> Self {
        Self { role, ..self }
    }

    /// The run this item ends, itself last: the thoughts one block draws.
    fn thoughts<'b>(&'b self, item: &'b Item) -> Vec<&'b Item> {
        self.run.iter().copied().chain([item]).collect()
    }
}

/// One item's block. `place` is where it sits among its neighbours — the item
/// before it, which a receipt joins, and the run of thoughts it may end;
/// `agents` the sub-sessions this transcript's calls spawned.
pub fn item_block(
    item: &Item,
    place: &Place<'_>,
    agents: &Agents<'_>,
    rows: &Rows<'_>,
    cue: Cue,
) -> Block {
    if matches!(&item.body, ItemBody::User { origin, .. } if origin.is_context()) {
        return Block::default();
    }
    if !matches!(place.role, crate::turns::Role::Addition)
        && crate::ask::receipts::source(item)
            .is_some_and(|id| rows.state.transcript.get(id).is_some())
    {
        return Block::default();
    }
    if let Some(block) = steps::part(item, &place.role, rows) {
        return if matches!(place.role, crate::turns::Role::Step { .. }) {
            pictured::under_the_words(
                item,
                block,
                if rows.opened.well(item) {
                    Fold::Open
                } else {
                    Fold::Shut
                },
                rows,
            )
        } else {
            block
        };
    }
    // Loose blocks retain the fold chosen by their pointer gesture.
    let fold = fold::fold_of(rows.opened.folds, item);
    let mut block = item_lines(item, place, agents, rows, cue, fold);
    if item.turn.is_some()
        && place
            .previous
            .is_some_and(|previous| previous.turn == item.turn)
        && matches!(item.body, ItemBody::Assistant { .. })
        && !block.lines.is_empty()
    {
        block.lines.insert(0, Line::default());
    }
    crate::ask::receipts::append(item, &mut block.lines, rows);
    pictured::under_the_words(item, block, fold, rows)
}

fn default_item_lines(
    item: &Item,
    place: &Place<'_>,
    agents: &Agents<'_>,
    rows: &Rows<'_>,
    cue: Cue,
    fold: Fold,
) -> Block {
    match &item.body {
        ItemBody::Unknown { .. } => Block::default(),
        ItemBody::User { parts, origin } => {
            if band::owns(item, rows.state) {
                band::draw(item, rows).into()
            } else {
                said::lines(item, parts, origin, place, fold, rows).into()
            }
        }
        ItemBody::Assistant { text } => {
            assistant(text, item.status, fold, rows, rows.arriving(item))
        }
        ItemBody::Reasoning { .. } => thinking::lines(item, place, fold, rows).into(),
        ItemBody::ToolCall { .. } => called(item, agents, fold, rows).into(),
        ItemBody::Shell { .. } => ran::lines(item, rows).into(),
        ItemBody::Action { name, args, result } => {
            action(item.status, name, args, result.as_ref(), rows, cue).into()
        }
        ItemBody::Compaction {
            before,
            after,
            window,
            ..
        } => notice::compacted(item, (*before, *after), *window, rows).into(),
        ItemBody::Rewind { dropped, .. } => rewound(*dropped, rows).into(),
        ItemBody::Interruption { marker, .. } => vec![indented(marker, theme::dim())].into(),
        ItemBody::Notice { level, text, .. } => speaks(
            theme::level(*level),
            vec![Line::from(Span::styled(text.clone(), theme::level(*level)))],
            rows,
        )
        .into(),
        ItemBody::QuestionAnswer {
            question, answer, ..
        } => answered(question, answer, rows).into(),
        ItemBody::PermissionReceipt {
            tool,
            decision,
            feedback,
            ..
        } => receipt(tool, *decision, feedback.as_deref(), place.previous, rows).into(),
        ItemBody::Asset { asset, label } => vec![indented(
            &format!("[{}]", label.clone().unwrap_or_else(|| asset.clone())),
            theme::dim(),
        )]
        .into(),
    }
}

/// A line of words where a mark's text starts, with no mark before it.
fn indented(text: &str, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::raw(" ".repeat(speaks_indent())),
        Span::styled(text.to_owned(), style),
    ])
}

/// The seam a rewind left.
fn rewound(dropped: u32, rows: &Rows<'_>) -> Vec<Line<'static>> {
    vec![rule(
        &format!("rewound, {dropped} items dropped"),
        rows.width,
    )]
}

/// Every call is its own row, a call that started or addressed a teammate in
/// that teammate's voice (design §3: a child is a row where it began) — but a call that
/// moved the task list draws no row at all, because the list under the
/// activity row is its row (M74, as Claude Code draws none).
fn called(item: &Item, agents: &Agents<'_>, fold: Fold, rows: &Rows<'_>) -> crate::views::Drawing {
    let ItemBody::ToolCall { output, quiet, .. } = &item.body else {
        return crate::views::Drawing::default();
    };
    let Some(step) = crate::turns::step(item, rows) else {
        return crate::views::Drawing::default();
    };
    if *quiet && !item.status.failed(output.as_deref()) {
        return quiet_call(output.as_deref(), rows);
    }
    let mut step = step;
    step.teammate = crate::turns::teammate::of(item, agents, rows);
    steps::step_block(item, &step, fold == Fold::Open, rows).into_text_drawing()
}

fn quiet_call(output: Option<&ToolOutput>, rows: &Rows<'_>) -> crate::views::Drawing {
    output
        .and_then(|output| output.display.as_ref())
        .map(|view| {
            crate::views::marked(
                view,
                rows.result_width(),
                &crate::views::Marks::at(rows.now),
            )
            .map_lines(|lines| step_body(lines, rows))
        })
        .unwrap_or_default()
}

/// A display or picture stays below its step until a detail well can own it.
fn step_body(body: Vec<Line<'static>>, rows: &Rows<'_>) -> Vec<Line<'static>> {
    under(Span::raw("     "), body, 5, rows.measured())
}

// ---- the two marks ------------------------------------------------------

/// Where the text after a mark starts: the mark at column 1, its text at 3
/// (design §4), as every row of the transcript has it.
fn speaks_indent() -> usize {
    theme::bullet().width() + 2
}

/// The `  ⎿  ` a result hangs from, and where its text starts: column 5.
fn connector() -> String {
    format!("  {}  ", theme::connector())
}

/// A block under a mark: the mark on its first row, its text at `indent` on
/// every row under it, wrapped so nothing overflows the measure.
fn under(
    mark: Span<'static>,
    body: Vec<Line<'static>>,
    indent: usize,
    width: usize,
) -> Vec<Line<'static>> {
    let inner = width.saturating_sub(indent).max(1);
    let mut out: Vec<Line<'static>> = Vec::new();
    for line in wrap::wrap_all(&body, inner) {
        let lead = match out.is_empty() {
            true => mark.clone(),
            false => Span::raw(" ".repeat(indent)),
        };
        let mut spans = vec![lead];
        spans.extend(line.spans);
        out.push(Line::from(spans));
    }
    out
}

/// What the model says and does.
fn speaks(style: Style, body: Vec<Line<'static>>, rows: &Rows<'_>) -> Vec<Line<'static>> {
    marked(theme::bullet(), style, body, rows)
}

/// A block under a glyph in the bullet's place. The glyph says what kind of
/// row it is and the style says what state it is in, which is why the two are
/// separate: a skill's mark still turns `good` when its call comes back.
fn marked(
    glyph: &str,
    style: Style,
    body: Vec<Line<'static>>,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    let mark = Span::styled(format!(" {glyph} "), style);
    under(mark, body, speaks_indent(), rows.measured())
}

/// What came back.
fn returns(body: Vec<Line<'static>>, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let mark = connector();
    let indent = mark.width();
    under(
        Span::styled(mark, theme::dim()),
        body,
        indent,
        rows.measured(),
    )
}

// ---- the kinds ----------------------------------------------------------

/// The answer arrives without surface pacing, with a tail blend and caret.
///
/// The words may name a picture themselves (M51). The tail is lit first: a row
/// of placeholder cells is not text, and cooling it would spend the colour the
/// picture's own number is carried in.
fn assistant(text: &str, status: ItemStatus, fold: Fold, rows: &Rows<'_>, arriving: bool) -> Block {
    let width = rows.measured().saturating_sub(5).min(100);
    let source = answer::closed(text, status);
    let mut written = markdown::mentioned(&source, width, rows.faces.names());
    answer::finish(&mut written.lines, status, width, rows.now, arriving);
    let body = pictured::in_the_words(written.lines, &written.images, fold, rows);
    Block {
        lines: body
            .lines
            .into_iter()
            .map(|mut line| {
                line.spans.insert(0, Span::raw("   "));
                line
            })
            .collect(),
        ..body
    }
}

/// Legacy child rows keep their own pulse until M200 supplies their step projection.
fn live_bullet(status: ItemStatus, failed: bool, rows: &Rows<'_>) -> Bullet {
    let moving = rows.now.motion && status == ItemStatus::Running;
    let style = if moving {
        theme::pulse(clock::breath(rows.now, PULSE))
    } else {
        bullet_style(status, failed)
    };
    Bullet {
        style,
        next: moving.then_some(rows.now.instant + clock::FRAME),
    }
}

#[must_use]
struct Bullet {
    style: Style,
    next: Option<Instant>,
}

impl Bullet {
    fn heading(self, body: Vec<Line<'static>>, rows: &Rows<'_>) -> crate::views::Drawing {
        crate::views::Drawing {
            lines: marked(theme::bullet(), self.style, body, rows),
            next: self.next,
        }
    }
}

/// `Read(Cargo.toml)`: the name bold, what it is about plain — and, for the
/// twelve frames after the call comes back, whatever its landing is doing to
/// the two of them.
fn signature(name: &str, about: &str, rows: &Rows<'_>, landing: Landing) -> Line<'static> {
    let mut spans = named(name, landing);
    let about = rows.shorten(about);
    if !about.is_empty() {
        spans.push(Span::styled(format!("({about})"), landing.about()));
    }
    Line::from(spans)
}

/// The name of the row: one span at rest, and one per cell while a light is
/// crossing it.
fn named(name: &str, landing: Landing) -> Vec<Span<'static>> {
    match landing {
        Landing::Settled => vec![Span::styled(name.to_string(), theme::bold())],
        Landing::Failed(come) => vec![Span::styled(
            name.to_string(),
            theme::cooling(come).patch(theme::bold()),
        )],
        Landing::Landed(come) => swept(name, come),
    }
}

/// The name with one light crossing it, cell by cell: the same sweep the box's
/// border runs when a line is sent, on the other side of the conversation.
fn swept(name: &str, come: f32) -> Vec<Span<'static>> {
    let cells = name.chars().count();
    name.chars()
        .enumerate()
        .map(|(at, glyph)| {
            Span::styled(
                glyph.to_string(),
                theme::landing(clock::sweep(come, at, cells)),
            )
        })
        .collect()
}

/// The bullet carries the state, and nothing else has to.
fn bullet_style(status: ItemStatus, failed: bool) -> Style {
    if failed {
        return theme::bad();
    }
    match status {
        ItemStatus::Running => theme::presence(),
        ItemStatus::Completed => theme::good(),
        ItemStatus::Failed => theme::bad(),
        ItemStatus::Pending | ItemStatus::Interrupted => theme::dim(),
    }
}

/// A sub-session is a row where it began: what it is, what it was asked, and
/// what it is doing — read from its own state, never copied into this one.
/// A long-running operation of the surface's own: login, reconnect, a team
/// starting. It reads as a tool row because that is what it is.
fn action(
    status: ItemStatus,
    name: &str,
    args: &Value,
    result: Option<&Value>,
    rows: &Rows<'_>,
    cue: Cue,
) -> Vec<Line<'static>> {
    let failed = status == ItemStatus::Failed;
    let mut out = speaks(
        bullet_style(status, failed),
        vec![signature(
            name,
            &as_text(args),
            rows,
            Landing::of(cue, failed, rows),
        )],
        rows,
    );
    if let Some(result) = result {
        out.extend(returns(plain(&as_text(result)), rows));
    }
    out
}

/// Strings travel verbatim; anything else as compact JSON.
fn as_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A question the model asked and the answer it was given.
fn answered(question: &str, answer: &str, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let mut out = speaks(
        theme::text().patch(theme::bold()),
        vec![Line::from(Span::styled(
            question.to_string(),
            theme::text(),
        ))],
        rows,
    );
    out.extend(returns(
        vec![Line::from(Span::styled(answer.to_string(), theme::dim()))],
        rows,
    ));
    out
}

/// The gate's answer, joined to the row that asked for it. The tool is named
/// only when the row above did not already name it.
fn receipt(
    tool: &str,
    decision: DecisionKind,
    feedback: Option<&str>,
    previous: Option<&Item>,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    let verdict = match decision {
        DecisionKind::AllowChangedSession => "you changed it first; yes for this session",
        DecisionKind::AllowChanged => "you changed it first",
        DecisionKind::Allow => "you: yes",
        DecisionKind::AllowSession => "you: yes for this session",
        DecisionKind::Deny => "you: no",
    };
    let said = match previous.is_some_and(|item| calls(item, tool)) {
        true => verdict.to_string(),
        false => format!("{tool} {verdict}"),
    };
    let text = match feedback {
        Some(feedback) => format!("{said} — {}", rows.shorten(feedback)),
        None => said,
    };
    returns(vec![Line::from(Span::styled(text, theme::dim()))], rows)
}

fn calls(item: &Item, tool: &str) -> bool {
    matches!(&item.body, ItemBody::ToolCall { name, .. } if name == tool)
}

/// What stands under the last turn once it is over and nothing runs: why it
/// failed, followed by any informational closing notice. Both are
/// derived from `last_turn` rather than kept as lines of the surface's own,
/// and neither belongs to an item, so this is the transcript's last block
/// rather than one of them: it scrolls with the answer it closes (M84,
/// 2026-09-08, user-directed — not a row pinned over the composer) and the
/// next turn takes it away.
pub fn closing(state: &SessionState, rows: &Rows<'_>) -> crate::views::Drawing {
    let mut out = live::draw(state, rows);
    for props in crate::component::closing::props(state) {
        out.append(crate::component::closing::draw(
            state,
            rows.pulled,
            &props,
            rows.width,
            rows.now,
        ));
    }
    out
}

pub(crate) fn component_closing(
    state: &SessionState,
    props: &bingo_sdk::component::Props,
    width: usize,
) -> Vec<Line<'static>> {
    let bingo_sdk::component::Props::InfoNotice(props) = props else {
        return Vec::new();
    };
    if props.command.as_deref() == Some(crate::component::signed_out::COMMAND) {
        return crate::component::signed_out::lines(state, &props.text, width);
    }
    let head = match state.last_status() {
        Some(bingo_sdk::TurnStatus::Failed { error }) => {
            error.code.as_str().to_lowercase().replace('_', " ")
        }
        _ => "error".into(),
    };
    let line = crate::layout::grid::fit(
        Line::from(vec![
            Span::styled(head, theme::bad()),
            Span::styled(theme::glyphs::get("sep"), theme::dim()),
            Span::styled(props.text.clone(), theme::text()),
        ]),
        width.saturating_sub(5),
    );
    let mut out = under(Span::raw("   "), vec![line], 3, width);
    if let Some(command) = &props.command {
        out.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(format!("/{command}"), theme::ash()),
        ]));
    }
    out
}

/// A divider from column 1 to the edge, with its reason in the left run.
fn rule(text: &str, width: usize) -> Line<'static> {
    let head = format!(" {0}{0}{0} {text} ", theme::rule());
    let tail = width.saturating_sub(head.width());
    Line::from(Span::styled(
        format!("{head}{}", theme::rule().repeat(tail)),
        theme::dim(),
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_permission_answer_leaves_a_receipt() {
        assert!(
            drawn(vec![receipt_item(
                "itm_1",
                "Bash",
                DecisionKind::Allow,
                None
            )])
            .join("\n")
            .contains("you: yes")
        );
        assert!(
            drawn(vec![receipt_item(
                "itm_1",
                "Bash",
                DecisionKind::AllowSession,
                None
            )])
            .join("\n")
            .contains("you: yes for this session")
        );
        assert!(
            drawn(vec![receipt_item(
                "itm_1",
                "Bash",
                DecisionKind::Deny,
                Some("keep it")
            )])
            .join("\n")
            .contains("you: no — keep it")
        );
    }

    use super::*;
    use crate::test_support::{
        agent_call, assistant, chosen_wells, completed, delivered, extended, folded, frame, item,
        loose, post, receipt_item, running_tool, scene, started, started_tool, tool, ts,
        user as person, voiced,
    };
    use bingo_sdk::{Event, ItemId, TurnId};

    #[test]
    fn an_upgraded_historical_agent_call_draws_a_tool_row() {
        let frame: bingo_sdk::Frame = serde_json::from_str(include_str!(
            "../../bingo-store-jsonl/fixtures/agent/upgraded.json"
        ))
        .unwrap();
        let Event::ItemCompleted { item } = frame.event else {
            panic!("completed call")
        };
        let rows = drawn(vec![item]).join("\n");
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| f.render_widget(ratatui::widgets::Paragraph::new(rows.as_str()), f.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("¶ read    Read note.md"), "{text}");
        assert!(text.contains("hello"), "{text}");
    }

    /// A member's own transcript: a session a model answers.
    fn drawn(items: Vec<Item>) -> Vec<String> {
        let state = stated(items);
        rendered_wells(&state, &chosen_wells(&state))
    }

    /// Turn-less calls use the same step geometry and keep their folded output.
    fn turnless(items: Vec<Item>) -> Vec<String> {
        drawn(items.into_iter().map(loose).collect())
    }

    /// The same items in the room's own transcript: a session nothing answers,
    /// under the room's name (ADR-0011 §1).
    fn drawn_in_room(items: Vec<Item>) -> Vec<String> {
        let mut state = stated(items);
        state.summary.title = Some("#design".to_string());
        state.summary.driver = bingo_sdk::Driver::Log;
        rendered(&state)
    }

    /// The same items in a surface with a skill mod's placed command row.
    fn drawn_knowing_the_skill(items: Vec<Item>) -> Vec<String> {
        rendered_with(
            &crate::test_support::with_skill_row(
                stated(items),
                "itm_1",
                "guide",
                "the wire format",
            ),
            &Folds::new(),
        )
    }

    /// Every item outside any turn, so each draws its own block: these tests
    /// pin each kind's grammar; turn rows are covered by `turn_tests`.
    fn stated(items: Vec<Item>) -> SessionState {
        folded(
            items
                .into_iter()
                .enumerate()
                .map(|(i, item)| frame(i as u64 + 1, Event::ItemCompleted { item }))
                .collect(),
        )
    }

    /// The transcript without the welcome box, which `welcome.rs` pins on its
    /// own: these tests are about the grammar under it.
    fn rendered(state: &SessionState) -> Vec<String> {
        rendered_with(state, &Folds::new())
    }

    /// A call on a memory file is drawn as remembering (M84): the verb for the
    /// tool, the memory's own name — and the same call on any other file is
    /// what it was.
    #[test]
    fn a_call_on_a_memory_file_is_drawn_as_remembering() {
        let published = frame(
            1,
            extended(
                "_bingo.context",
                "memory",
                serde_json::json!({"user": "/data/memory/user", "project": "/data/memory/web-2bf6c26a7362cd1f"}),
            ),
        );
        let calls = [
            tool(
                "itm_1",
                "Read",
                serde_json::json!({"file_path": "/data/memory/web-2bf6c26a7362cd1f/the-build.md"}),
                Some(ToolOutput::text("run cargo test")),
                ItemStatus::Completed,
            ),
            tool(
                "itm_2",
                "Write",
                serde_json::json!({"file_path": "/data/memory/user/MEMORY.md", "content": "- …"}),
                Some(ToolOutput::text("written")),
                ItemStatus::Completed,
            ),
            tool(
                "itm_3",
                "Read",
                serde_json::json!({"file_path": "src/lib.rs"}),
                Some(ToolOutput::text("mod a;")),
                ItemStatus::Completed,
            ),
        ];
        let mut frames = vec![published];
        frames.extend(
            calls
                .into_iter()
                .map(loose)
                .enumerate()
                .map(|(i, item)| frame(i as u64 + 2, Event::ItemCompleted { item })),
        );
        let mut state = folded(frames);
        for (item, title, badge) in [
            ("itm_1", "Recall from memory", "the-build"),
            ("itm_2", "Write memory", "MEMORY.md"),
        ] {
            state = crate::test_support::with_row(
                state,
                bingo_sdk::slot::Slot::Row {
                    item: ItemId::from_raw(item),
                    mark: None,
                    title: Some(title.into()),
                    badge: Some(badge.into()),
                    caption: None,
                    body: None,
                    actions: vec![],
                },
            );
        }
        let lines = rendered(&state);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Recall from memory  the-build")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("Write memory  MEMORY.md")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("read    src/lib.rs")),
            "{lines:?}"
        );
        assert!(!lines.iter().any(|l| l.contains("read(/data")), "{lines:?}");
    }

    /// A call that moved the task list is no row: the list under the activity
    /// row is its row (M74). One that failed, or went to a board, still is —
    /// each has something on this screen the list cannot say.
    #[test]
    fn a_task_call_draws_no_row_unless_the_list_cannot_say_what_it_did() {
        let ok = Some(ToolOutput::text("Created #1: write the plan"));
        let quiet = drawn(vec![
            person("itm_1", "note the plan"),
            tool(
                "itm_2",
                "TaskCreate",
                serde_json::json!({"subject": "write the plan"}),
                ok.clone(),
                ItemStatus::Completed,
            ),
            assistant("itm_3", "Noted.", ItemStatus::Completed),
        ]);
        assert!(!quiet.iter().any(|l| l.contains("TaskCreate")), "{quiet:?}");
        assert_eq!(
            quiet,
            drawn(vec![
                person("itm_1", "note the plan"),
                assistant("itm_3", "Noted.", ItemStatus::Completed),
            ]),
            "no gap and no step where the call was"
        );

        let board = drawn(vec![tool(
            "itm_2",
            "TaskCreate",
            serde_json::json!({"subject": "write the plan", "in": "#design"}),
            ok,
            ItemStatus::Completed,
        )]);
        assert!(
            board
                .iter()
                .any(|l| l.contains("• taskcreate  write the plan")),
            "{board:?}"
        );

        let failed = drawn(vec![tool(
            "itm_2",
            "TaskUpdate",
            serde_json::json!({"id": 9}),
            Some(ToolOutput::error("No task #9")),
            ItemStatus::Completed,
        )]);
        assert!(
            failed.iter().any(|l| l.contains("No task #9")),
            "{failed:?}"
        );
    }

    fn rendered_with(state: &SessionState, folds: &Folds) -> Vec<String> {
        rendered_choices(state, folds, &Default::default())
    }

    fn rendered_wells(
        state: &SessionState,
        wells: &std::collections::BTreeMap<TurnId, ItemId>,
    ) -> Vec<String> {
        rendered_choices(state, &Default::default(), wells)
    }

    fn rendered_choices(
        state: &SessionState,
        folds: &Folds,
        wells: &std::collections::BTreeMap<TurnId, ItemId>,
    ) -> Vec<String> {
        let mut blocks = crate::blocks::Blocks::default();
        let pictures = Decoded::default();
        let linked = Linked::default();
        let mut rows = Rows::of(state, 60, folds, &pictures, &linked, scene().1);
        rows.opened.wells = Some(wells);
        let height = blocks.sync(state, &Agents::new(), &rows);
        let mut rows: Vec<String> = blocks
            .window(0, height)
            .iter()
            .map(|line| line.to_string().trim_end().to_string())
            .collect();
        if rows.first().is_some_and(String::is_empty) {
            rows.remove(0);
        }
        rows
    }

    /// The whole of ADR-0034 in one transcript: a room's activity is the
    /// room's, so a member's own — the holder's included — draws none of it,
    /// and what the person typed is still a block of their own.
    #[test]
    fn a_members_transcript_draws_none_of_its_rooms_machinery() {
        assert_eq!(
            drawn(vec![
                voiced(
                    delivered(
                        "itm_1",
                        "room",
                        None,
                        "#design has posts you have not read."
                    ),
                    bingo_sdk::Voice::Aside
                ),
                delivered(
                    "itm_2",
                    "contributor:rooms",
                    None,
                    "[#design, since you last read]\nscout: found it",
                ),
                voiced(
                    post("itm_3", "reviewer", "two nits, otherwise fine"),
                    bingo_sdk::Voice::Aside
                ),
                person("itm_4", "thanks"),
            ]),
            vec!["▍  thanks".to_string()],
            "the nudge, the reading and a copied post are the room's"
        );
    }

    /// The set is exactly the two origins the rooms plugin writes: a message
    /// from a peer names a conversation of its own and still draws, and every
    /// other contributor still speaks.
    #[test]
    fn only_the_rooms_own_origins_are_kept_out_of_a_member() {
        assert_eq!(
            drawn(vec![delivered(
                "itm_1",
                "agent",
                Some("scout"),
                "Two nits, else fine."
            )]),
            vec!["▍  scout  Two nits, else fine.".to_string()],
        );
        assert_eq!(
            drawn(vec![delivered(
                "itm_2",
                "contributor:experience:recall",
                None,
                "you have seen this before"
            )]),
            Vec::<String>::new(),
        );
    }

    #[test]
    fn internal_context_never_appears_as_a_transcript_row() {
        let text = format!(
            "Memory index\n{}",
            (0..40).map(|n| format!("fact-{n}\n")).collect::<String>()
        );
        let state = stated(vec![
            person("itm_ask", "Show the current state"),
            delivered("itm_context", "contributor:context:memory", None, &text),
            delivered("itm_peer", "agent", Some("reviewer"), "Review completed"),
            crate::test_support::assistant(
                "itm_answer",
                "Here is the result",
                ItemStatus::Completed,
            ),
        ]);
        let hidden = Folds::from([(ItemId::from_raw("itm_context"), Fold::Open)]);
        for lines in [rendered(&state), rendered_with(&state, &hidden)] {
            let text = lines.join("\n");
            assert!(text.contains("Show the current state"), "{text}");
            assert!(text.contains("Review completed"), "{text}");
            assert!(text.contains("Here is the result"), "{text}");
            assert!(!text.contains("Memory index"), "{text}");
            assert!(!text.contains("fact-"), "{text}");
            assert!(!text.contains("context:memory"), "{text}");
        }
        let (mut ui, now) = scene();
        let screen = crate::test_support::draw_sized(80, 24, &state, &mut ui, now);
        assert!(screen.contains("Show the current state"), "{screen}");
        assert!(screen.contains("Review completed"), "{screen}");
        assert!(!screen.contains("Memory index"), "{screen}");
        assert!(!screen.contains("fact-"), "{screen}");
        assert_eq!(
            state.transcript.items().len(),
            4,
            "presentation must not erase the journal"
        );
    }

    #[test]
    fn internal_context_does_not_supply_a_session_brief_or_rewind_prompt() {
        let context = delivered(
            "itm_context",
            "contributor:runtime",
            None,
            "Hidden runtime state",
        );
        let state = stated(vec![context]);
        assert_eq!(crate::tree::brief(&state), None);
        assert!(
            bingo_sdk::transcript::turns(&state.transcript)
                .iter()
                .all(|turn| turn.headline.is_none())
        );
    }

    /// The room's own transcript is where its activity is read, and every post
    /// it holds is drawn there — the name first, then the whole message.
    #[test]
    fn a_room_reads_as_a_conversation_does() {
        assert_eq!(
            drawn_in_room(vec![post(
                "itm_1",
                "scout",
                "found it\n\nit was the cursor all along",
            )]),
            vec![
                " ● scout: found it".to_string(),
                String::new(),
                "   it was the cursor all along".to_string(),
            ],
            "no `⎿`, nothing dim, nothing folded: a message is read whole"
        );
    }

    /// A post nobody signed came from the session the room hangs under, and it
    /// is read under the name every seat on the roster reads it under — the
    /// same row as anybody else's, because in the room it is anybody else's.
    #[test]
    fn an_unsigned_post_is_the_holders_and_says_so() {
        assert_eq!(
            drawn_in_room(vec![person("itm_1", "then let us ship it")]),
            vec![" ● parent: then let us ship it"],
        );
    }

    #[test]
    fn a_second_line_of_yours_stays_on_the_bar_under_the_first() {
        assert_eq!(
            drawn(vec![person("itm_1", "one\ntwo")]),
            vec!["▍  one".to_string(), "▍  two".to_string()],
        );
    }

    /// Only a person surface speaks unstamped: every other origin an older
    /// journal carries without a voice — the subsystems that exist, the room's,
    /// and one that never will — is a marked line, never the person's bar. A
    /// room's stamped aside is read in the room and draws no row here.
    #[test]
    fn every_unstamped_origin_but_a_persons_is_a_marked_line() {
        for surface in [
            "agent",
            "bash",
            "command",
            "schedule",
            "room",
            "wake",
            "brand-new",
        ] {
            assert_eq!(
                drawn(vec![delivered("itm_1", surface, None, "it ended")]),
                vec![" ▍ it ended".to_string()],
                "{surface} is quiet"
            );
        }
        assert!(
            drawn(vec![voiced(
                delivered("itm_1", "room", None, "it ended"),
                bingo_sdk::Voice::Aside
            )])
            .is_empty(),
            "and the room's own is not drawn in a member at all"
        );
        for loud in bingo_sdk::PERSON_SURFACES {
            assert_eq!(
                drawn(vec![delivered("itm_1", loud, None, "it ended")]),
                vec!["▍  it ended".to_string()],
                "{loud} is a person surface"
            );
        }
    }

    /// A wake opens a turn the way typing does, and is not typing: it is on
    /// the bar, in the gutter its own word stands in, and every line under
    /// the first hangs past that word rather than past a `>` (ADR-0019 §8).
    #[test]
    fn a_wake_is_a_line_on_the_bar_under_a_word_of_its_own() {
        assert_eq!(
            drawn(vec![voiced(
                delivered(
                    "itm_1",
                    "wake",
                    None,
                    "check the build again\nand say what you find",
                ),
                bingo_sdk::Voice::Note
            )]),
            vec![
                "▍  wake-up  check the build again".to_string(),
                "▍  and say what you find".to_string(),
            ],
        );
    }

    /// A job reporting in: the first line says what happened, and what one
    /// does about it hangs under the row the way a result does.
    #[test]
    fn a_finished_job_reads_as_a_tool_row_does() {
        assert_eq!(
            drawn(vec![delivered(
                "itm_1",
                "bash",
                None,
                "Background job ab12cd34 exited with code 1 after 2m.\n\
                 `BashOutput` with id \"ab12cd34\" reads what it wrote.",
            )]),
            vec![
                " ▍ Background job ab12cd34 exited with code 1 after 2m.".to_string(),
                "  ⎿  `BashOutput` with id \"ab12cd34\" reads what it wrote.".to_string(),
            ],
        );
    }

    /// A skill's prompt as the kernel journals it: the typed line, then the
    /// body the command expanded to.
    fn skill_prompt() -> Item {
        delivered(
            "itm_1",
            "command",
            None,
            "/guide the wire format\n\n\
             Base directory for this skill: /skills/guide\n\n\
             Read this before answering about bingo itself.",
        )
    }

    /// The user-directed rule of 2026-09-02: a skill is one row however it was
    /// asked for. What the person typed stays in the item — a rewind reads it
    /// back off there — but the row is the run, and the page it expanded to
    /// hangs under it as any result does.
    #[test]
    fn a_typed_skill_reads_as_the_run_it_is() {
        assert_eq!(
            drawn_knowing_the_skill(vec![skill_prompt()]),
            vec![
                " ❖ Skill(guide) the wire format".to_string(),
                "  ⎿  Base directory for this skill: /skills/guide".to_string(),
                String::new(),
                "     Read this before answering about bingo itself.".to_string(),
            ],
        );
    }

    /// The catalogue is what says a name is a skill. A command it files
    /// elsewhere — and every command, before the catalogue has landed — keeps
    /// the row it always had: the line somebody typed.
    #[test]
    fn a_command_the_catalogue_calls_no_skill_is_the_line_that_was_typed() {
        assert_eq!(
            drawn(vec![skill_prompt()]),
            vec![
                " ▍ /guide the wire format".to_string(),
                "  ⎿  Base directory for this skill: /skills/guide".to_string(),
                String::new(),
                "     Read this before answering about bingo itself.".to_string(),
            ],
        );
    }

    /// The other door, on the same row: the model's own call names the skill
    /// in `input.name` and hands it `input.arguments`. The row is not written
    /// out again here — it is asserted to be the one above, which is the whole
    /// of what "one skill row" means.
    #[test]
    fn the_models_skill_call_draws_a_step_row() {
        let called = drawn(vec![tool(
            "itm_1",
            "Skill",
            serde_json::json!({"name": "guide", "arguments": "the wire format"}),
            Some(ToolOutput::text(
                "Base directory for this skill: /skills/guide",
            )),
            ItemStatus::Completed,
        )]);
        assert_eq!(
            called,
            vec![
                "   • skill   guide",
                "",
                "       Base directory for this skill: /skills/guide",
                ""
            ]
        );
        assert!(!called.join("\n").contains('⎿'));
    }

    #[test]
    fn a_skill_with_no_arguments_is_the_signature_alone() {
        assert_eq!(
            drawn(vec![tool(
                "itm_1",
                "Skill",
                serde_json::json!({"name": "guide"}),
                None,
                ItemStatus::Running,
            )]),
            vec!["   ⠋ skill   guide".to_string()],
        );
    }

    /// A call that names no skill is the tool it is: the fallback never
    /// invents a name for the screen.
    #[test]
    fn a_skill_call_that_names_nothing_is_drawn_as_a_tool_row() {
        let lines = drawn(vec![tool(
            "itm_1",
            "Skill",
            serde_json::json!({"arguments": "the wire format"}),
            None,
            ItemStatus::Completed,
        )]);
        assert_eq!(
            lines[..],
            ["   • skill   the wire format", "", "       no output", ""],
            "{lines:?}"
        );
    }

    /// A message from another session says who sent it, as a room's post does.
    #[test]
    fn an_agents_message_names_the_agent_on_the_marked_line() {
        assert_eq!(
            drawn(vec![delivered(
                "itm_1",
                "agent",
                Some("reviewer"),
                "done, two nits"
            )]),
            vec!["▍  reviewer  done, two nits".to_string()],
        );
    }

    /// A long notice folds the way a long result does — and offers no key,
    /// because `ctrl+o` reaches a result and this is not one.
    #[test]
    fn a_long_notice_folds_without_promising_a_key() {
        let body: String = (1..=9).map(|i| format!("\nline {i}")).collect();
        let drawn = drawn(vec![delivered(
            "itm_1",
            "schedule",
            None,
            &format!("the nightly run is in{body}"),
        )]);
        assert_eq!(drawn.len(), OUTPUT_ROWS + 2);
        assert_eq!(drawn[0], " ▍ the nightly run is in");
        assert_eq!(drawn.last().map(String::as_str), Some("     … +4 lines"));
    }

    #[test]
    fn the_model_speaks_after_a_bold_bullet_and_keeps_its_indent() {
        assert_eq!(
            drawn(vec![assistant(
                "itm_1",
                "alpha bravo charlie delta echo foxtrot golf hotel india juliet",
                ItemStatus::Completed,
            )]),
            vec![
                "   alpha bravo charlie delta echo foxtrot golf hotel india".to_string(),
                "   juliet".to_string(),
            ],
            "a wrapped answer hangs under its own bullet"
        );
    }

    #[test]
    fn a_tool_row_names_the_call_and_hangs_its_result_under_it() {
        let output = ToolOutput::text("Read 3 lines");
        let lines = drawn(vec![tool(
            "itm_1",
            "Read",
            serde_json::json!({"file_path": "/tmp/project/Cargo.toml"}),
            Some(output),
            ItemStatus::Completed,
        )]);
        assert_eq!(
            lines[..],
            ["   • read    Cargo.toml", "", "       Read 3 lines", ""],
            "the path is named from the session's own directory"
        );
    }

    /// A turn opened to be read hangs what a step brought back whole: the
    /// fold that cuts a long result is a row's, and the turn's line is the
    /// fold here.
    #[test]
    fn an_opened_turn_hangs_a_long_result_whole() {
        let output = ToolOutput::text((1..=9).map(|i| format!("line {i}\n")).collect::<String>());
        let lines = drawn(vec![tool(
            "itm_1",
            "Read",
            serde_json::json!({"file_path": "src/lib.rs"}),
            Some(output),
            ItemStatus::Completed,
        )]);
        assert_eq!(lines.len(), 1 + 2 + 9, "{lines:?}");
        assert_eq!(lines[10], "       line 9");
    }

    #[test]
    fn a_long_tool_result_keeps_only_its_step_row() {
        let output = ToolOutput::text((1..=9).map(|i| format!("line {i}\n")).collect::<String>());
        let drawn = turnless(vec![tool(
            "itm_1",
            "Read",
            serde_json::json!({"file_path": "src/lib.rs"}),
            Some(output),
            ItemStatus::Completed,
        )]);
        assert_eq!(drawn.len(), 1);
        assert!(!drawn[0].contains("ctrl+o"));
    }

    #[test]
    fn a_running_tool_shows_the_last_rows_of_its_tail() {
        let state = folded(vec![started_tool(
            1,
            loose(running_tool("itm_1", "Bash", "one\ntwo\nthree\nfour")),
        )]);
        assert_eq!(
            rendered(&state),
            vec![
                "   ⠋ bash    cargo test".to_string(),
                "             four".to_string()
            ],
        );
    }

    /// A thought, once it is one: how long it took, and what it was.
    fn thought_item(text: &str, seconds: i64) -> Item {
        thought_at("itm_1", text, seconds)
    }

    /// The same, named: a run is several of them, so each needs its own id.
    fn thought_at(id: &str, text: &str, seconds: i64) -> Item {
        let mut item = item(
            id,
            ItemStatus::Completed,
            ItemBody::Reasoning {
                text: text.into(),
                provider_metadata: Default::default(),
            },
        );
        item.completed_at = Some(ts() + jiff::SignedDuration::from_secs(seconds));
        item
    }

    /// A thought as it is being had: no `completed_at`, and as much text as
    /// the deltas have carried so far.
    fn thinking_item(text: &str) -> Item {
        thinking_at("itm_1", text)
    }

    fn thinking_at(id: &str, text: &str) -> Item {
        item(
            id,
            ItemStatus::Running,
            ItemBody::Reasoning {
                text: text.into(),
                provider_metadata: Default::default(),
            },
        )
    }

    /// A run of thoughts is one thought (M79, user-directed: 会有这种连续的
    /// 思考 我感觉可以合并成一个). The provider closes a reasoning item and
    /// opens the next as the model goes on thinking, so ten of them in a row
    /// were ten blocks; a person reads them as one thought that took as long
    /// as the whole run. One heading, the summed time, the run's texts joined.
    #[test]
    fn a_run_of_thoughts_draws_as_one_thought() {
        assert_eq!(
            turnless(vec![
                thought_at("itm_1", "The manifest first.", 2),
                thought_at("itm_2", "Then the crate map.", 3),
            ]),
            vec![
                " ▏ Thought for 5s".to_string(),
                "  ⎿  The manifest first.".to_string(),
                "     Then the crate map.".to_string(),
            ],
            "one block, one heading, the last two rows of the joined text"
        );
    }

    /// The run's heading is the last thought's state: while the newest of them
    /// is still being had the whole run says so, and the tail is the newest of
    /// what the run has thought.
    #[test]
    fn a_run_whose_last_thought_is_still_being_had_is_thinking() {
        assert_eq!(
            turnless(vec![
                thought_at("itm_1", "The manifest first.", 2),
                thought_at("itm_2", "Then the crate map.", 3),
                thinking_at("itm_3", "The plan after that."),
            ]),
            vec![
                " ▏ Thinking…".to_string(),
                "  ⎿  Then the crate map.".to_string(),
                "     The plan after that.".to_string(),
            ],
        );
    }

    /// Adjacent is adjacent: anything the model did between two thoughts keeps
    /// them apart, because what it did is what a person read between them.
    #[test]
    fn a_call_between_two_thoughts_keeps_them_two_blocks() {
        let lines = drawn(vec![
            thought_at("itm_1", "The manifest first.", 2),
            tool(
                "itm_2",
                "Read",
                serde_json::json!({"file_path": "Cargo.toml"}),
                Some(ToolOutput::text("Read 3 lines")),
                ItemStatus::Completed,
            ),
            thought_at("itm_3", "Then the crate map.", 3),
        ]);
        assert_eq!(
            lines[..],
            ["   • read    Cargo.toml", "", "       Read 3 lines", "",],
            "{lines:?}"
        );
    }

    /// A run is long: ten thoughts of half a minute each is what the journal
    /// behind M79 held, so the heading reads a clock rather than a count of
    /// seconds nobody divides in their head.
    #[test]
    fn a_run_of_minutes_reads_as_minutes_and_seconds() {
        let drawn = turnless(vec![
            thought_at("itm_1", "The manifest first.", 70),
            thought_at("itm_2", "Then the crate map.", 131),
        ]);
        assert_eq!(
            drawn.first().map(String::as_str),
            Some(" ▏ Thought for 3m 21s"),
            "{drawn:?}"
        );
    }

    #[test]
    fn thinking_streams_and_then_holds_the_rows_it_streamed() {
        assert_eq!(
            turnless(vec![thinking_item("the manifest")]),
            vec![" ▏ Thinking…".to_string(), "  ⎿  the manifest".to_string()],
            "a thought being had is read as it is thought"
        );
        assert_eq!(
            turnless(vec![thought_item("The manifest first.", 2)]),
            vec![
                " ▏ Thought for 2s".to_string(),
                "  ⎿  The manifest first.".to_string(),
            ],
            "and once it is over it holds: the heading's words are the whole of the change"
        );
    }

    /// The close moves nothing (2026-09-06, user-directed): a thought that is
    /// over holds the rows it held on its last delta, so the heading's words
    /// are the whole of what changes and the conversation above it stays where
    /// it was. Three shapes of text, because the cut is a tail measured in
    /// rows and each of them wraps to those rows differently.
    #[test]
    fn a_thought_that_closes_holds_the_rows_it_was_streaming() {
        let paragraph = "The manifest first, because the lockfile only says what the \
                         manifest already asked for, and then the crate map, which is \
                         the one place the whole of the layering is written down.";
        let broken = "The manifest first.\n\nThen the crate map, which is the one place \
                      the layering is written down.\n\nThe plan after that: it says \
                      which of the two is allowed to move.";
        for text in ["the manifest", paragraph, broken] {
            let streaming = turnless(vec![thinking_item(text)]);
            let over = turnless(vec![thought_item(text, 2)]);
            assert_eq!(
                streaming.len(),
                over.len(),
                "the block keeps its height: {streaming:?} / {over:?}"
            );
            assert_eq!(
                streaming[1..],
                over[1..],
                "and every row under the heading: {streaming:?} / {over:?}"
            );
        }
    }

    /// Two rows, the newest of what has arrived — the same tail a running tool
    /// wears (§6) and two rows of it rather than three, because a thought is
    /// read past.
    #[test]
    fn a_thought_being_had_streams_two_rows_of_its_newest_text() {
        assert_eq!(
            turnless(vec![thinking_item("first\nsecond\nthird\nfourth")]),
            vec![
                " ▏ Thinking…".to_string(),
                "  ⎿  third".to_string(),
                "     fourth".to_string(),
            ],
        );
    }

    /// A paragraph is one logical line and any number of rows, so the cut
    /// counts rows: the row and [`THOUGHT_ROWS`] under it however wide the
    /// prose runs, and what they hold is the end of it.
    #[test]
    fn a_thought_of_one_long_paragraph_still_streams_two_rows() {
        let paragraph = "The manifest first, because the lockfile only says what the \
                         manifest already asked for, and then the crate map, which is \
                         the one place the whole of the layering is written down, and \
                         only after both of them the plan that says which may move.";
        let drawn = turnless(vec![thinking_item(paragraph)]);
        assert_eq!(drawn.len(), 1 + THOUGHT_ROWS, "{drawn:?}");
        assert!(
            drawn
                .last()
                .is_some_and(|row| row.ends_with("which may move.")),
            "the newest end of it: {drawn:?}"
        );
    }

    /// Only the newest output line hangs under a running step, cut to its width.
    #[test]
    fn a_running_tools_long_lines_are_cut_to_rows() {
        let progress = "Compiling bingo-surface-tui v0.1.0 (/tmp/project/crates/bingo-surface-tui)\n\
                        Compiling bingo-core v0.1.0 (/tmp/project/crates/bingo-core)\n\
                        Compiling bingo-provider-fake v0.1.0 (/tmp/project/crates/bingo-provider-fake)";
        let state = folded(vec![started_tool(
            1,
            loose(running_tool("itm_1", "Bash", progress)),
        )]);
        let drawn = rendered(&state);
        assert_eq!(drawn.len(), 2, "{drawn:?}");
        assert!(drawn[1].starts_with("             Compiling bingo-provider-fake"));
    }

    /// The bug the row-wise cut was made for (2026-09-06, user-reported): the
    /// transcript is anchored at its foot, so a block whose height moves moves
    /// everything above it. Over every word of a thought that runs through two
    /// paragraph breaks, the block is never taller than the row and its two,
    /// and never shorter than it has already been.
    #[test]
    fn a_streaming_thought_never_changes_the_height_of_its_block() {
        let text = "The manifest first, because the lockfile only says what the manifest \
                    already asked for.\n\nThen the crate map, which is the one place the \
                    layering is written down, and the plan after it, which says which of \
                    the two is allowed to move.\n\nOnly then the code, and only the file \
                    the plan names; anything else is a second reading of the same three \
                    facts.";
        let mut tallest = 0;
        for stop in word_ends(text) {
            let so_far = &text[..stop];
            let height = turnless(vec![thinking_item(so_far)]).len();
            assert!(height <= 1 + THOUGHT_ROWS, "{height} rows after {so_far:?}");
            assert!(height >= tallest, "{height} rows after {so_far:?}");
            tallest = height;
        }
        assert_eq!(tallest, 1 + THOUGHT_ROWS);
    }

    /// Where each word of a growing text ends, which is where one delta of it
    /// would have left the reader.
    fn word_ends(text: &str) -> Vec<usize> {
        text.char_indices()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect()
    }

    /// The three states a thought that is over has, in the order a click walks
    /// them (§7). It is the one block with a shut, and the shut is on the far
    /// side of the whole rather than where the thought was met.
    #[test]
    fn a_finished_thought_holds_its_tail_then_opens_whole_then_shuts() {
        let text: String = (1..=9).map(|i| format!("step {i}\n")).collect();
        let state = folded(vec![frame(
            1,
            Event::ItemCompleted {
                item: loose(thought_item(&text, 3)),
            },
        )]);
        let at = |fold| {
            let folds: Folds = [(ItemId::from_raw("itm_1"), fold)].into_iter().collect();
            rendered_with(&state, &folds)
        };
        assert_eq!(
            at(Fold::Peek),
            vec![
                " ▏ Thought for 3s".to_string(),
                "  ⎿  step 8".to_string(),
                "     step 9".to_string(),
            ],
            "the hold is the tail it ended on, and it promises nothing"
        );
        let open = at(Fold::Open);
        assert_eq!(open.len(), 10, "the row and every step: {open:?}");
        assert_eq!(open.last().map(String::as_str), Some("     step 9"));
        assert_eq!(
            at(Fold::Shut),
            vec![" ▏ Thought for 3s".to_string()],
            "and one more click puts the thought away"
        );
    }

    /// Nothing thought yet, and nothing to fold: the row alone, as an empty
    /// thought stays once it is over.
    #[test]
    fn a_thought_with_nothing_in_it_yet_is_the_row_alone() {
        assert_eq!(
            turnless(vec![thinking_item("")]),
            vec![" ▏ Thinking…".to_string()],
        );
    }

    /// ADR-0035 §4: an ACP agent runs its own tools and journals each finished
    /// call as a reasoning item whose metadata is the whole call. It draws as
    /// the call it was — the same bullet, signature and folded result every
    /// other call wears — and the heading the item's text carries is what the
    /// row replaces rather than repeats.
    #[test]
    fn an_agents_own_call_draws_as_a_tool_row_and_not_as_a_thought() {
        let lines = drawn(vec![agent_call(
            "itm_1",
            "read Read src/lib.rs (1 - 50)done\npub mod wire;",
            serde_json::json!({
                "external": true,
                "toolCallId": "toolu_01Read",
                "title": "Read src/lib.rs (1 - 50)",
                "kind": "read",
                "status": "completed",
                "content": [
                    { "type": "content",
                      "content": { "type": "text", "text": "pub mod wire;" } }
                ],
                "rawInput": { "file_path": "/tmp/project/src/lib.rs" },
                "rawOutput": { "lines": 1 }
            }),
        )]);
        assert_eq!(
            lines[..],
            [
                "   ¶ read    Read src/lib.rs (1 - 50)",
                "",
                "       pub mod wire;",
                ""
            ],
            "{lines:?}"
        );
    }

    /// A call still being made carries no metadata — the provider writes it
    /// when the block closes — so it reads exactly as it did before the row
    /// existed: the heading, under `▏ Thinking…`.
    #[test]
    fn an_agents_call_still_running_reads_as_the_text_it_is() {
        assert_eq!(
            turnless(vec![thinking_item("read Read src/lib.rs (1 - 50)")]),
            vec![
                " ▏ Thinking…".to_string(),
                "  ⎿  read Read src/lib.rs (1 - 50)".to_string(),
            ],
        );
    }

    /// And a thought is a thought: no metadata, no row. The mark is the whole
    /// of what tells the two apart.
    #[test]
    fn a_thought_without_the_mark_is_untouched() {
        assert_eq!(
            turnless(vec![thought_item("The manifest first.", 2)]),
            vec![
                " ▏ Thought for 2s".to_string(),
                "  ⎿  The manifest first.".to_string(),
            ],
        );
        assert_eq!(
            turnless(vec![agent_call(
                "itm_1",
                "The manifest first.",
                serde_json::json!({ "title": "not a call" }),
            )]),
            vec![
                " ▏ Thought for 1s".to_string(),
                "  ⎿  The manifest first.".to_string(),
            ],
            "a namespace without the flag is somebody's private note, not a call"
        );
    }

    /// Every field of the mark but the flag is optional, so a journal an older
    /// build wrote still draws a row — the name it can state, and nothing else.
    #[test]
    fn a_thin_mark_from_an_older_build_still_draws_a_row() {
        let lines = drawn(vec![agent_call(
            "itm_1",
            "tool something",
            serde_json::json!({ "external": true }),
        )]);
        assert_eq!(
            lines[..],
            ["   • other   Tool", "", "       no output", ""],
            "{lines:?}"
        );
    }

    /// A call that failed says so where every other row says it: the bullet.
    /// Nothing about the row is invented for it — the status is the agent's.
    #[test]
    fn a_failed_call_wears_the_row_a_failed_tool_wears() {
        let state = stated(vec![agent_call(
            "itm_1",
            "tool toolu_04Bashfailed\nno such file",
            serde_json::json!({
                "external": true,
                "toolCallId": "toolu_04Bash",
                "title": "toolu_04Bash",
                "status": "failed",
                "content": [
                    { "type": "content",
                      "content": { "type": "text", "text": "no such file" } }
                ]
            }),
        )]);
        let lines = rendered_wells(&state, &chosen_wells(&state));
        assert_eq!(
            lines[..],
            [
                "   • other   toolu_04Bash                           failed",
                "",
                "       no such file",
                "",
            ],
            "{lines:?}"
        );
        let item = state.transcript.items().first().expect("the call");
        assert_eq!(fold::fold_of(&Folds::new(), item), Fold::Peek);
    }

    /// However long a thought was, what a person meets is the row and the two
    /// rows it ended on — the rows it was already wearing, and never the
    /// five-row cut of somebody else's working it stopped being.
    #[test]
    fn a_long_thought_holds_the_two_rows_it_ended_on() {
        let text: String = (1..=99).map(|i| format!("step {i}\n")).collect();
        assert_eq!(
            turnless(vec![thought_item(&text, 3)]),
            vec![
                " ▏ Thought for 3s".to_string(),
                "  ⎿  step 98".to_string(),
                "     step 99".to_string(),
            ],
        );
    }

    /// Redacted thinking, or a turn the provider summarised nothing of: the
    /// row alone, with no fold under it and no key promised.
    #[test]
    fn an_empty_thought_is_the_row_and_nothing_else() {
        assert_eq!(
            turnless(vec![thought_item("", 1)]),
            vec![" ▏ Thought for 1s".to_string()],
        );
        assert_eq!(thought(&thought_item("   \n", 1)), None);
        assert_eq!(thought(&thought_item("why", 1)), Some("why"));
    }

    #[test]
    fn a_receipt_joins_the_row_that_asked_for_it() {
        assert_eq!(
            turnless(vec![
                tool(
                    "itm_1",
                    "Edit",
                    serde_json::json!({"file_path": "src/lib.rs"}),
                    None,
                    ItemStatus::Failed,
                ),
                receipt_item("itm_2", "Edit", DecisionKind::Deny, Some("use cargo clean")),
            ]),
            vec![
                "   • edit    src/lib.rs                             failed".to_string(),
                "  ⎿  you: no — use cargo clean".to_string(),
            ],
            "no blank line, and the tool is named once"
        );
    }

    #[test]
    fn a_receipt_with_no_call_above_it_names_its_own_tool() {
        assert_eq!(
            drawn(vec![receipt_item(
                "itm_1",
                "Edit",
                DecisionKind::AllowSession,
                None
            )]),
            vec!["  ⎿  Edit you: yes for this session".to_string()],
        );
    }

    #[test]
    fn a_failed_turn_names_its_error_above_the_hint() {
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                completed(
                    "trn_1",
                    bingo_sdk::TurnStatus::Failed {
                        error: bingo_sdk::KernelError::new(
                            bingo_sdk::ErrorCode::ProviderUnavailable,
                            "no route to the provider",
                        ),
                    },
                ),
            ),
        ]);
        assert_eq!(
            rendered(&state).last().map(String::as_str),
            Some("   provider unavailable · no route to the provider")
        );
    }

    /// The widest row a state draws at `width`, past the welcome box.
    fn widest_row(state: &SessionState, width: usize, measure: Option<usize>) -> usize {
        let mut blocks = crate::blocks::Blocks::default();
        let folds = Folds::new();
        let pictures = Decoded::default();
        let linked = Linked::default();
        let rows = Rows::of(state, width, &folds, &pictures, &linked, scene().1).measuring(measure);
        let height = blocks.sync(state, &Agents::new(), &rows);
        blocks
            .window(0, height)
            .iter()
            .map(|line| line.to_string().trim_end().width())
            .max()
            .unwrap_or(0)
    }

    /// Inside a person's narrower region, prose keeps its measure from
    /// column 3 (ADR-0097 §3).
    #[test]
    fn prose_keeps_its_measure_inside_a_persons_narrower_region() {
        let state = folded(vec![frame(
            1,
            Event::ItemCompleted {
                item: assistant("itm_1", &"word ".repeat(60), ItemStatus::Completed),
            },
        )]);
        assert_eq!(widest_row(&state, 200, None), 102);
        assert_eq!(widest_row(&state, 200, Some(60)), 57);
    }

    #[test]
    fn an_opened_result_shows_every_line() {
        let output = ToolOutput::text((1..=9).map(|i| format!("line {i}\n")).collect::<String>());
        let items = vec![tool(
            "itm_1",
            "Read",
            serde_json::json!({"file_path": "src/lib.rs"}),
            Some(output),
            ItemStatus::Completed,
        )];
        let frames = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| frame(i as u64 + 1, Event::ItemCompleted { item }))
            .collect();
        let state = folded(frames);
        let wells = [(TurnId::from_raw("trn_1"), ItemId::from_raw("itm_1"))]
            .into_iter()
            .collect();
        let rows = rendered_wells(&state, &wells).join("\n");
        assert!(rows.contains("line 9"), "{rows}");
        assert!(!rows.contains("+4 lines"), "{rows}");
    }

    #[test]
    fn a_tab_in_a_result_runs_to_the_next_stop_of_eight() {
        assert_eq!(
            output::expand_tabs("     1\t[package]"),
            "     1  [package]"
        );
        assert_eq!(output::expand_tabs("a\tb\tc"), "a       b       c");
        assert_eq!(output::expand_tabs("no tabs"), "no tabs");
    }
    #[test]
    fn an_opaque_item_draws_no_terminal_cells() {
        let known = assistant("itm_known", "known answer", ItemStatus::Completed);
        let body = serde_json::from_str(include_str!(
            "../../bingo-sdk/fixtures/record/unknown/item.json"
        ))
        .unwrap();
        let opaque = item("itm_opaque", ItemStatus::Completed, body);
        let before = stated(vec![known.clone()]);
        let after = stated(vec![known, opaque]);
        assert_eq!(
            drawn(
                after
                    .transcript
                    .items()
                    .iter()
                    .map(|i| i.as_ref().clone())
                    .collect()
            ),
            rendered(&before)
        );
        let (mut ui, now) = scene();
        assert_eq!(
            crate::test_support::render(&before, &mut ui, now),
            crate::test_support::render(&after, &mut ui, now)
        );
        assert_eq!(after.transcript.items().len(), 2, "retained, but not drawn");
    }
}

#[cfg(test)]
mod ember_tests;

#[cfg(test)]
mod turn_tests;
