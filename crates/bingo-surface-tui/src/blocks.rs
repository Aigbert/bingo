//! The transcript as blocks: one rendered block per item, kept between
//! frames.
//!
//! The reducer is still the only history — a block is a *memo* of
//! [`crate::transcript`]'s rendering of one item at one width in one look and
//! one graphics mode, thrown away the moment any of them changes, never a
//! second copy of what the item says. An item that has finished is drawn once
//! and then only cloned; the one that is still arriving is the only one that
//! is drawn again, because it is the only one that can differ. A call that
//! spawned a session is drawn again whenever the child's state moves, since
//! its row is read from that state; a call that sent a shell to the
//! background, when the shell ends, for the same reason.
//!
//! Scrolling asks for a window of lines, which is why the blocks are kept
//! rather than the flat transcript: a window is walked in blocks and cut to
//! the row, so what it hands back is always an exact slice of the whole.

use std::time::{Duration, Instant};

use bingo_sdk::{Item, ItemBody, ItemId, ItemStatus, Seq, SessionState, TurnId};
use ratatui::text::Line;

use crate::clock::Now;
use crate::fold::{self, Fold};
use crate::graphics::Picture;
use crate::transcript::{self, Cue, Place, Rows};
use crate::tree::Agents;
use crate::turns::{Role, Roles};

/// What can change about an item's block while it is on the screen. A
/// terminal item's revision never moves again, so its block is rendered once.
#[derive(Clone, Debug, PartialEq)]
struct Revision {
    receipts: Vec<ItemId>,
    component: (u64, u64),
    render_config: serde_json::Value,
    status: ItemStatus,
    arriving: bool,
    /// Names and room data affect prose even when an item's bytes stay put.
    names: Vec<crate::markdown::mentions::Name>,
    room: Option<Seq>,
    /// How much the body holds. Anything that grows a block grows this: the
    /// text of an answer, a tail, an output.
    size: usize,
    /// Where the child this call spawned is, when it spawned one: its row is
    /// read from the child's state, so the child's seq is the row's revision.
    agent: Option<Seq>,
    /// How much of the block is shown: what `ctrl+o` and a click both set, and
    /// the start its kind has until they do.
    fold: Fold,
    /// How many pictures the words named have been read in so far (M51). A
    /// block that drew a chip for one is a different block once it has
    /// arrived, and the arrival is the only thing that says so — it is not in
    /// the item, which has not changed a byte.
    linked: u64,
    /// Live row placements revise a block even when the item itself is unchanged.
    slots: Vec<bingo_sdk::slot::Slot>,
    /// How many thoughts of its run are above this one, and whether it is the
    /// last of them (M79). A run of thoughts draws as one block, hung on its
    /// last item, so the run growing by one is a different rendering of that
    /// block — and the item that *was* last is a different rendering too: it
    /// draws nothing now, and the entry has to go the frame it stops.
    run: usize,
    last: bool,
    /// The look it was drawn in. A style is baked into a `Line`, and the
    /// terminal may change which palette a token is worth under a running
    /// surface (M71): the lines are then a memo of a drawing nobody would make
    /// again. It is here rather than beside the width because a look changes
    /// what a block *says* about itself and not what it is — the entry, and the
    /// landing it is in the middle of, are kept and only drawn again.
    look: crate::theme::Theme,
    /// Whether, and at what cell, the terminal draws pictures. A late answer
    /// can turn them on under a running surface, and a block drawn before it
    /// holds the chip where the picture now goes.
    graphics: crate::graphics::Graphics,
    /// The part it plays in its turn, including its resolved tool look.
    role: Role,
    tool: Option<bingo_sdk::component::Step>,
    band: Option<crate::transcript::band::Revision>,
    shell: bool,
}

fn revision(
    item: &Item,
    place: &Place<'_>,
    agent: Option<&SessionState>,
    fold: Fold,
    rows: &Rows<'_>,
) -> Revision {
    let request = crate::component::items::id(item);
    Revision {
        receipts: receipts(item, rows),
        component: (
            // The band and its clock share one block, so either reply revises it.
            rows.pulled.map_or(0, |memo| {
                memo.version(&request)
                    .max(memo.version(&item.id.to_string()))
                    .max(memo.version(&format!("{}:duration", item.id)))
            }),
            rows.state
                .invalidated
                .get(&request)
                .copied()
                .unwrap_or_default(),
        ),
        render_config: rows.state.config.kernel["render"].clone(),
        status: item.status,
        arriving: matches!(place.role, Role::Own)
            && matches!(item.body, ItemBody::Assistant { .. })
            && rows.arriving(item),
        names: rows.names.clone(),
        room: (rows.state.summary.driver == bingo_sdk::Driver::Log).then_some(rows.state.seq),
        size: size(&item.body),
        agent: agent.map(|child| child.seq),
        fold,
        run: place.run.len(),
        last: !place.joined,
        linked: rows.linked.answers(),
        slots: rows.placed_on(&item.id).cloned().collect(),
        look: crate::theme::current(),
        graphics: crate::graphics::chosen(),
        role: place.role.clone(),
        band: crate::transcript::band::revision(item, rows),
        shell: crate::transcript::ran::latest(rows.state).is_some_and(|shell| shell.id == item.id),
        tool: match &item.body {
            ItemBody::ToolCall { name, input, .. } if !matches!(place.role, Role::Step { .. }) => {
                Some(crate::tool_rows::resolve(
                    name,
                    input,
                    &rows.tools,
                    crate::theme::bullet().is_ascii(),
                ))
            }
            _ => None,
        },
    }
}

fn receipts(item: &Item, rows: &Rows<'_>) -> Vec<ItemId> {
    let band = item.turn.is_some()
        && transcript::band::owns(item, rows.state)
        && transcript::band::first(item, rows);
    rows.answers
        .to(&item.id)
        .iter()
        .map(|receipt| receipt.id.clone())
        .chain(
            rows.turns
                .get(&item.turn)
                .filter(|_| band)
                .into_iter()
                .flatten()
                .filter(|other| {
                    matches!(
                        other.body,
                        ItemBody::User { .. } | ItemBody::PermissionReceipt { .. }
                    )
                })
                .map(|other| other.id.clone()),
        )
        .collect()
}

/// How much of the body is on the screen. Anything that grows a block grows
/// this: the text of an answer, a tail, an output — and a thought's text, which
/// M34-B left out because the row said `▏ Thinking…` whatever the deltas
/// carried and nothing under it was drawn from them. A thought now streams its
/// tail under that row (§6), so a delta does change the rows and the block has
/// to be drawn again for it.
fn size(body: &ItemBody) -> usize {
    match body {
        ItemBody::Assistant { text } => text.len(),
        ItemBody::Reasoning { text, .. } => text.len(),
        ItemBody::ToolCall {
            output, progress, ..
        } => {
            output.as_ref().map(|o| o.parts.len() + 1).unwrap_or(0)
                + progress.as_ref().map(String::len).unwrap_or(0)
        }
        ItemBody::Action { result, .. } => result.is_some() as usize,
        _ => 0,
    }
}

/// Where a block is in its own motion (§6). A block is drawn from the item and
/// the clock alone; this is the clock's half — when this rendering arrived,
/// when it stops changing, whether it arrived by finishing, and whether what
/// is held is a frame of that motion rather than its rest.
#[derive(Clone, Copy, Debug)]
struct Motion {
    since: Instant,
    until: Instant,
    flipped: bool,
    moved: bool,
}

impl Motion {
    fn new(since: Instant, settles: Duration, flipped: bool) -> Self {
        Self {
            since,
            until: since + settles,
            flipped,
            moved: !settles.is_zero(),
        }
    }

    fn moving(&self, now: Instant) -> bool {
        now < self.until
    }

    /// Whether the block has to be drawn again: it is still moving, or it has
    /// stopped and what is held is the last frame of the motion rather than
    /// the resting form that replaces it.
    fn redraw(&self, now: Instant) -> bool {
        self.moving(now) || self.moved
    }

    /// The same motion, as of a rendering taken at `now`. A rendering taken
    /// after the motion has stopped *is* the resting form, so it pays off the
    /// frame [`Motion::redraw`] was owed; a debt never marked paid keeps the
    /// whole surface awake for the rest of the run. It is the same sample the
    /// cue is drawn from, so a block can never come to rest while the frame
    /// held of it is still a flashing one.
    fn drawn(self, now: Instant) -> Self {
        Self {
            moved: self.moving(now),
            ..self
        }
    }

    fn cue(&self, now: Instant) -> Cue {
        Cue {
            since: self.since,
            flip: self.flipped && self.moving(now),
        }
    }
}

/// How long a block goes on changing after it is drawn: the frames a landed
/// call's own words move for, the comet tail of an answer still arriving, and
/// no time at all for anything that is simply there.
fn settles(item: &Item, flip: bool, moving: bool) -> Duration {
    if !moving || matches!(item.body, ItemBody::Assistant { .. }) {
        return Duration::ZERO;
    }
    if flip {
        return transcript::LANDING;
    }
    Duration::ZERO
}

fn cue_deadline(
    agent: Option<&SessionState>,
    role: &Role,
    motion: Motion,
    now: Now,
) -> Option<Instant> {
    if !matches!(role, Role::Own) {
        return None;
    }
    let child_moves = agent.is_some_and(|child| {
        matches!(
            bingo_collab::lifecycle::Lifecycle::of_state(child),
            Some(
                bingo_collab::lifecycle::Lifecycle::Working
                    | bingo_collab::lifecycle::Lifecycle::Asking
            )
        )
    });
    if (child_moves && now.motion) || motion.redraw(now.instant) {
        return Some(now.instant + crate::clock::FRAME);
    }
    None
}

struct Entry {
    id: ItemId,
    revision: Revision,
    /// A receipt hangs from the row above it: no blank row before it.
    joins: bool,
    /// The turn it was said or done in: no blank row parts a turn's own rows.
    turn: Option<TurnId>,
    lines: Vec<Line<'static>>,
    /// The pictures these lines stand for (§5). They are kept with the lines
    /// because they are the same rendering: a block drawn once and cloned
    /// ever after would otherwise forget, on its second frame, what its own
    /// placeholder cells are placeholders for.
    pictures: Vec<Picture>,
    /// The destinations these words named (M51), for the same reason.
    wanted: Vec<String>,
    motion: Motion,
    next: Option<Instant>,
}

/// One run of lines in the stacked transcript: the welcome box, an item's
/// block, or the failed turn's line — with whether a blank row goes before it
/// and whose it is.
struct Segment<'a> {
    gap: bool,
    id: Option<&'a ItemId>,
    lines: &'a [Line<'static>],
}

/// The rendered transcript of one session at one width. The blocks are kept
/// in transcript order and matched to the items by position, so a frame that
/// changed nothing costs one comparison per item and not one allocation.
#[derive(Default)]
pub struct Blocks {
    width: usize,
    /// The measure prose was wrapped to inside that width (`tui.measure`,
    /// design §7). It is beside the width because it is the other half of the
    /// geometry a block is a rendering of, and neither half is on the item.
    measure: Option<usize>,
    blocks: Vec<Entry>,
    /// The transcript's height in wrapped lines, counted while the blocks are
    /// brought up to date rather than walked for again.
    height: usize,
    /// What closes the last turn — its failure, its cost — which belongs to
    /// no item.
    tail: Vec<Line<'static>>,
    tail_next: Option<Instant>,
    /// How many blocks have been drawn since this cache was made. Tests
    /// distinguish memo hits from visible motion refreshes.
    renders: usize,
    roles: Roles,
}

impl Blocks {
    /// Walk block coordinates once when collecting visible component requests.
    pub fn spans(&self) -> impl Iterator<Item = (&ItemId, std::ops::Range<usize>)> {
        let mut at = 0;
        self.segments().filter_map(move |segment| {
            at += usize::from(segment.gap);
            let start = at;
            at += segment.lines.len();
            segment.id.map(|id| (id, start..at))
        })
    }

    pub fn closing_start(&self) -> usize {
        self.height.saturating_sub(self.tail.len())
    }
    /// Bring the cache up to date with the state and answer with the height of
    /// the whole transcript, in wrapped lines.
    pub fn sync(&mut self, state: &SessionState, agents: &Agents<'_>, rows: &Rows<'_>) -> usize {
        self.relaid(rows);
        self.roles = Roles::of(agents, rows);
        let mut kept = 0;
        for (at, item) in state.transcript.items().iter().enumerate() {
            let role = self.roles.of_item(&item.id).clone();
            let place = Place::of(state.transcript.items(), at).playing(role);
            kept += self.block(kept, item, &place, agents, rows, false);
        }
        // Whatever is left behind the last item was rewound away.
        self.blocks.truncate(kept);
        let tail = transcript::closing(state, rows);
        self.tail = tail.lines;
        self.tail_next = tail.next;
        self.height = self.measure();
        self.height
    }

    /// Drop every block the frame is no longer laid out for. A block is a
    /// rendering at one geometry, and the geometry is both numbers: the width
    /// of the region and the measure prose is wrapped to inside it.
    fn relaid(&mut self, rows: &Rows<'_>) {
        if self.width == rows.width && self.measure == rows.measure {
            return;
        }
        self.blocks.clear();
        self.width = rows.width;
        self.measure = rows.measure;
    }

    #[cfg(test)]
    pub fn moving(&self) -> bool {
        self.blocks.iter().any(|entry| entry.motion.moved)
    }

    fn visible(&self, from: usize, height: usize) -> impl Iterator<Item = usize> + '_ {
        let mut y = 0;
        self.segments()
            .enumerate()
            .filter_map(move |(index, segment)| {
                y += usize::from(segment.gap);
                let start = y;
                y += segment.lines.len();
                (height > 0
                    && segment.id.is_some()
                    && (y > from || (segment.lines.is_empty() && start >= from))
                    && start < from.saturating_add(height))
                .then_some(index)
            })
    }

    /// Only visible expired drawings are sampled again; history keeps its memo.
    pub fn refresh(
        &mut self,
        state: &SessionState,
        agents: &Agents<'_>,
        rows: &Rows<'_>,
        from: usize,
        height: usize,
    ) {
        let visible: Vec<_> = self.visible(from, height).collect();
        for index in visible.into_iter().rev() {
            let entry = &self.blocks[index];
            if entry.next.is_none_or(|at| at > rows.now.instant) {
                continue;
            }
            let Some(&at) = rows.positions.get(&entry.id) else {
                continue;
            };
            let item = &state.transcript.items()[at];
            let place = Place::of(state.transcript.items(), at)
                .playing(self.roles.of_item(&item.id).clone());
            self.block(index, item, &place, agents, rows, true);
        }
        self.height = self.measure();
    }

    pub fn next_change(&self, from: usize, height: usize) -> Option<Instant> {
        if height == 0 {
            return None;
        }
        let tail = (self.closing_start() < from.saturating_add(height)
            && self.height.max(self.closing_start() + 1) > from)
            .then_some(self.tail_next)
            .flatten();
        self.visible(from, height)
            .filter_map(|index| self.blocks[index].next)
            .chain(tail)
            .min()
    }

    /// Keep the block at `at` when it is still this item's, else draw it.
    /// Answers with how many blocks the item now occupies: one, or none when
    /// it has nothing to say.
    fn block(
        &mut self,
        at: usize,
        item: &Item,
        place: &Place<'_>,
        agents: &Agents<'_>,
        rows: &Rows<'_>,
        refresh: bool,
    ) -> usize {
        let now = rows.now.instant;
        let agent = agents.get(&item.id).copied();
        let revision = revision(item, place, agent, fold::fold_of(rows.folds, item), rows);
        let held = self.blocks.get(at).filter(|entry| entry.id == item.id);
        let same = held.is_some_and(|entry| entry.revision == revision);
        let occupied = held.is_some();
        // An item that has only just finished is not yet terminal for this
        // cache: it has a frame of flashing left to do, and a frame after that
        // to settle into.
        if same && !refresh {
            return 1;
        }
        let motion = self.motion(held, item, &revision, rows.now);
        self.renders += 1;
        let drawn = transcript::item_block(item, place, agents, rows, motion.cue(now));
        let next = drawn
            .next
            .into_iter()
            .chain(cue_deadline(agent, &place.role, motion, rows.now))
            .min();
        if drawn.lines.is_empty() && next.is_none() {
            // An item with nothing to say is not a block at all — and one that
            // said something until this frame gives its slot back with its
            // rendering, or the stale rows would stand and a blank row with
            // them (a thought a newer one joined, M79). Drawing it again every
            // frame costs a match and no allocation, which is what a block
            // that is not there is worth.
            if occupied {
                self.blocks.remove(at);
            }
            return 0;
        }
        let entry = Entry {
            id: item.id.clone(),
            revision,
            joins: transcript::joins_the_row_above(item),
            turn: item.turn.clone(),
            lines: drawn.lines,
            pictures: drawn.pictures,
            wanted: drawn.wanted,
            motion,
            next,
        };
        match self.blocks.get_mut(at) {
            Some(slot) if slot.id == item.id => *slot = entry,
            _ => self.blocks.insert(at, entry),
        }
        1
    }

    /// The clock this rendering is drawn against: the one it already had while
    /// nothing about it changed — settled against this frame, which is the one
    /// paying off whatever it still owed — and a new one the moment it did.
    fn motion(&self, held: Option<&Entry>, item: &Item, revision: &Revision, now: Now) -> Motion {
        let was = held.filter(|entry| &entry.revision == revision);
        if let Some(entry) = was {
            return entry.motion.drawn(now.instant);
        }
        let finished = held.is_some_and(|entry| !entry.revision.status.is_terminal())
            && item.status.is_terminal();
        Motion::new(now.instant, settles(item, finished, now.motion), finished)
    }

    /// The part each item plays in its turn, as the last sync drew it.
    pub fn roles(&self) -> &Roles {
        &self.roles
    }

    /// Where an item's block sits in the transcript: its first line, and the
    /// line just after it — where a card the item asked for hangs from.
    /// `None` when the item has no block.
    pub fn span(&self, item: &ItemId) -> Option<(usize, usize)> {
        let mut y = 0;
        for segment in self.segments() {
            y += usize::from(segment.gap);
            if segment.id == Some(item) {
                return Some((y, y + segment.lines.len()));
            }
            y += segment.lines.len();
        }
        None
    }

    /// The item whose block holds a transcript line, which is what a click in
    /// the transcript lands on.
    pub fn at(&self, line: usize) -> Option<ItemId> {
        let mut y = 0;
        for segment in self.segments() {
            y += usize::from(segment.gap);
            if (y..y + segment.lines.len()).contains(&line) {
                return segment.id.cloned();
            }
            y += segment.lines.len();
        }
        None
    }

    /// The whole transcript's height in wrapped lines, as the last [`sync`]
    /// counted it.
    ///
    /// [`sync`]: Blocks::sync
    pub fn height(&self) -> usize {
        self.height
    }

    fn measure(&self) -> usize {
        self.segments()
            .map(|segment| usize::from(segment.gap) + segment.lines.len())
            .sum()
    }

    /// The lines `[from, from + height)` of the whole transcript.
    pub fn window(&self, from: usize, height: usize) -> Vec<Line<'static>> {
        let mut out: Vec<Line<'static>> = Vec::with_capacity(height);
        let mut y = 0;
        for segment in self.segments() {
            let rows = usize::from(segment.gap) + segment.lines.len();
            if y + rows <= from {
                y += rows;
                continue;
            }
            if segment.gap {
                if y >= from {
                    out.push(Line::default());
                }
                y += 1;
            }
            for line in segment.lines {
                if out.len() == height {
                    return out;
                }
                if y >= from {
                    out.push(line.clone());
                }
                y += 1;
            }
            if out.len() == height {
                return out;
            }
        }
        out
    }

    /// Every picture the transcript is holding, in transcript order: what the
    /// terminal is asked to hold ([`crate::graphics::Stored`]). Derived from
    /// the blocks rather than remembered beside them, so a rewind that drops
    /// an item drops its picture with it.
    pub fn pictures(&self) -> Vec<Picture> {
        self.blocks
            .iter()
            .flat_map(|entry| entry.pictures.iter().cloned())
            .collect()
    }

    /// Every destination the transcript's own words named (M51), for the run
    /// to read in the ones this session has not got
    /// ([`crate::graphics::linked`]). Derived from the blocks for the same
    /// reason [`Blocks::pictures`] is: a rewind that drops an item stops
    /// anything being fetched for it.
    pub fn wanted(&self) -> Vec<String> {
        self.blocks
            .iter()
            .flat_map(|entry| entry.wanted.iter().cloned())
            .collect()
    }

    /// The last `rows` lines as plain text: what is printed back into the
    /// shell's own screen when the surface leaves (design §3).
    ///
    /// A picture's cells are not text. Printed into the shell's own screen
    /// they would be a row of glyphs no font has, standing for a picture the
    /// terminal has already been told to forget — so those rows are left
    /// behind with the alternate screen they were drawn on.
    pub fn tail(&self, rows: usize) -> Vec<String> {
        let height = self.height();
        let from = height.saturating_sub(rows);
        self.window(from, height - from)
            .iter()
            .map(|line| line.to_string().trim_end().to_string())
            .filter(|line| !line.contains(crate::graphics::kitty::PLACEHOLDER))
            .collect()
    }

    #[cfg(test)]
    pub fn renders(&self) -> usize {
        self.renders
    }

    /// Every segment in transcript order. A blank row separates a segment
    /// from whatever is above it — never from the top of the transcript, never
    /// a receipt from the row it answers, and never one row of a turn from
    /// another of the same turn.
    fn segments(&self) -> impl Iterator<Item = Segment<'_>> {
        let mut above: Option<&Entry> = None;
        let blocks = self.blocks.iter().map(move |entry| {
            let gap = !entry.lines.is_empty() && above.is_some_and(|above| parted(above, entry));
            if !entry.lines.is_empty() {
                above = Some(entry);
            }
            Segment {
                gap,
                id: Some(&entry.id),
                lines: entry.lines.as_slice(),
            }
        });
        let opened = !self.blocks.is_empty();
        let tail = (!self.tail.is_empty()).then_some(Segment {
            gap: opened,
            id: None,
            lines: self.tail.as_slice(),
        });
        blocks.chain(tail)
    }
}

/// Whether a blank row goes between two blocks: they are not one turn's, and
/// the lower one does not answer the upper.
fn parted(above: &Entry, below: &Entry) -> bool {
    let one_turn = below.turn.is_some() && below.turn == above.turn;
    !below.joins && !one_turn
}

impl std::fmt::Debug for Blocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Blocks")
            .field("width", &self.width)
            .field("blocks", &self.blocks.len())
            .field("renders", &self.renders)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::{Event, ItemStatus};

    #[test]
    fn later_events_keep_an_older_band_revision() {
        let mut first = user("old", "first turn");
        first.turn = Some(TurnId::from_raw("old-turn"));
        let mut state = folded(vec![
            frame(1, Event::ItemCompleted { item: first }),
            frame(2, started("trn_1")),
            frame(
                3,
                Event::ItemCompleted {
                    item: user("new", "second turn"),
                },
            ),
        ]);
        let mut blocks = cache();
        let now = later(scene().1, 2000);
        sync_at(&mut blocks, &state, 80, now);
        let revision = blocks.blocks[0].revision.clone();
        let newest = blocks.blocks[1].revision.clone();
        blocks.blocks[0].lines = vec![Line::from("memo retained")];
        state.apply(&frame(
            4,
            Event::ItemStarted {
                item: running_tool("step", "Inspect", "tail"),
            },
        ));
        sync_at(&mut blocks, &state, 80, later(now, 1000));
        assert_eq!(blocks.blocks[0].revision, revision);
        assert_eq!(blocks.blocks[0].lines[0].to_string(), "memo retained");
        assert_ne!(blocks.blocks[1].revision, newest);
        assert!(blocks.blocks[1].lines[0].to_string().contains("3s"));
    }

    fn cache() -> Blocks {
        Blocks::default()
    }

    /// `n` answers, each its own turn's, so a blank row parts every two.
    fn many(n: usize) -> SessionState {
        let mut state = state();
        state.transcript = (0..n)
            .map(|i| Item {
                turn: Some(TurnId::from_raw(format!("trn_{i}"))),
                ..assistant(
                    &format!("itm_{i}"),
                    &format!("Answer {i} — a line of prose that wraps a little."),
                    ItemStatus::Completed,
                )
            })
            .collect();
        state
    }

    fn sync(blocks: &mut Blocks, state: &SessionState, width: usize) -> usize {
        sync_at(blocks, state, width, scene().1)
    }

    fn sync_at(blocks: &mut Blocks, state: &SessionState, width: usize, now: Now) -> usize {
        let folds = crate::fold::Folds::new();
        let pictures = crate::graphics::Decoded::default();
        let linked = crate::graphics::Linked::default();
        let rows = Rows::of(state, width, &folds, &pictures, &linked, now);
        let height = blocks.sync(state, &Agents::new(), &rows);
        blocks.refresh(state, &Agents::new(), &rows, 0, height);
        blocks.height()
    }

    #[test]
    fn a_finished_item_is_drawn_once_and_scrolling_draws_nothing() {
        let state = many(200);
        let mut blocks = cache();
        let height = sync(&mut blocks, &state, 80);
        assert_eq!(blocks.renders(), 200);

        for from in (0..height).step_by(97) {
            blocks.window(from, 40);
        }
        sync(&mut blocks, &state, 80);
        assert_eq!(
            blocks.renders(),
            200,
            "a transcript that did not change is not drawn again"
        );
    }

    /// The row under a call that sent a shell to the background is read from
    /// the running set, not from the item, so the set's answer has to be part
    /// of what the block is a rendering of — or the row would say `Running`
    /// for as long as the transcript stood (M75).
    #[test]
    fn a_background_shell_keeps_its_landed_step_when_it_ends() {
        let started = bingo_sdk::ToolOutput {
            parts: vec![bingo_sdk::ContentPart::text("Started it")],
            is_error: false,
            display: Some(bingo_sdk::View::Custom {
                kind: "job".into(),
                data: serde_json::json!({ "id": "job_ab12cd34", "command": "sleep 45" }),
                fold: "Started in the background as job_ab12cd34".into(),
            }),
            upshot: None,
        };
        let set = serde_json::json!({
            "kind": "table", "headers": ["job", "command", "since"],
            "rows": [["job_ab12cd34", "sleep 45", "10:22:07"]],
        });
        let mut state = folded(vec![
            frame(
                1,
                Event::ItemCompleted {
                    item: tool(
                        "itm_1",
                        "Bash",
                        serde_json::json!({ "command": "sleep 45", "background": true }),
                        Some(started),
                        ItemStatus::Completed,
                    ),
                },
            ),
            frame(
                2,
                signalled(
                    "bingo.tools.bash",
                    "jobs",
                    crate::test_lanes::shell_payload(set, "itm_1"),
                ),
            ),
        ]);
        let mut blocks = cache();
        sync(&mut blocks, &state, 80);
        sync(&mut blocks, &state, 80);
        assert_eq!(
            blocks.renders(),
            1,
            "the shell runs on, nothing drawn again"
        );

        state.apply(&frame(
            3,
            signalled("bingo.tools.bash", "jobs", serde_json::Value::Null),
        ));
        sync(&mut blocks, &state, 80);
        assert_eq!(
            blocks.renders(),
            1,
            "the notice carries completion; the step stays in background"
        );
        sync(&mut blocks, &state, 80);
        assert_eq!(blocks.renders(), 1, "and it remains a memo");
    }

    /// A block is drawn once and cloned ever after, so a picture read in
    /// after its block was drawn would never reach the screen unless the
    /// answer's arrival were part of what a block is a rendering of (M51).
    #[test]
    fn a_picture_read_in_after_its_block_makes_it_draw_again() {
        let state = many(3);
        let mut blocks = cache();
        let folds = crate::fold::Folds::new();
        let pictures = crate::graphics::Decoded::default();
        let mut linked = crate::graphics::Linked::default();
        let now = scene().1;
        let sync = |blocks: &mut Blocks, linked: &crate::graphics::Linked| {
            let rows = Rows::of(&state, 80, &folds, &pictures, linked, now);
            blocks.sync(&state, &Agents::new(), &rows);
        };
        sync(&mut blocks, &linked);
        sync(&mut blocks, &linked);
        assert_eq!(blocks.renders(), 3, "nothing changed, nothing drawn again");

        assert!(linked.take("x.png"));
        sync(&mut blocks, &linked);
        assert_eq!(blocks.renders(), 3, "a read in flight changes no row");

        linked.answered(crate::graphics::linked::Answer {
            dest: "x.png".into(),
            result: Err("not found".into()),
        });
        sync(&mut blocks, &linked);
        assert_eq!(blocks.renders(), 6, "the answer landing draws them again");
        sync(&mut blocks, &linked);
        assert_eq!(blocks.renders(), 6, "and only once");
    }

    #[test]
    fn a_width_change_drops_every_block_once() {
        let state = many(20);
        let mut blocks = cache();
        sync(&mut blocks, &state, 80);
        sync(&mut blocks, &state, 100);
        assert_eq!(blocks.renders(), 40, "a new width is a new rendering");
        sync(&mut blocks, &state, 100);
        assert_eq!(blocks.renders(), 40, "and only one");
    }

    #[test]
    fn the_item_that_is_still_arriving_is_the_only_one_drawn_again() {
        let mut state = many(3);
        state.transcript.apply(&bingo_sdk::Event::ItemCompleted {
            item: assistant("itm_live", "half a s", ItemStatus::Running),
        });
        let mut blocks = cache();
        let now = scene().1;
        sync_at(&mut blocks, &state, 80, now);
        assert_eq!(blocks.renders(), 4);
        sync_at(
            &mut blocks,
            &state,
            80,
            later(now, crate::clock::FRAME.as_millis() as i64),
        );
        assert_eq!(blocks.renders(), 4, "arrival adds no pacing");
        state.transcript.apply(&bingo_sdk::Event::ItemCompleted {
            item: assistant("itm_live", "half a second", ItemStatus::Running),
        });
        sync_at(&mut blocks, &state, 80, later(now, 40));
        assert_eq!(blocks.renders(), 5, "one block, not four");
    }

    #[test]
    fn a_window_is_an_exact_slice_of_the_whole_transcript() {
        let state = many(40);
        let mut blocks = cache();
        let height = sync(&mut blocks, &state, 60);
        let whole: Vec<String> = blocks
            .window(0, height)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(whole.len(), height);
        for (from, rows) in [(0usize, 5usize), (1, 5), (7, 13), (height - 3, 9)] {
            let window: Vec<String> = blocks
                .window(from, rows)
                .iter()
                .map(ToString::to_string)
                .collect();
            let want = &whole[from..(from + rows).min(height)];
            assert_eq!(window, want, "window from {from}");
        }
    }

    #[test]
    fn a_block_knows_which_lines_are_its_own() {
        let state = many(3);
        let mut blocks = cache();
        sync(&mut blocks, &state, 60);
        let first = ItemId::from_raw("itm_0");
        let second = ItemId::from_raw("itm_1");
        let top = 0;
        assert_eq!(blocks.span(&first), Some((top, top + 1)));
        assert_eq!(
            blocks.span(&second),
            Some((top + 2, top + 3)),
            "past the blank row"
        );
        assert_eq!(blocks.at(top), Some(first));
        assert_eq!(blocks.at(top + 1), None, "the blank row is nobody's");
        assert_eq!(blocks.at(top + 2), Some(second));
        assert_eq!(blocks.at(999), None);
    }

    #[test]
    fn a_blank_row_separates_the_blocks_and_never_opens_the_transcript() {
        let state = many(2);
        let mut blocks = cache();
        let height = sync(&mut blocks, &state, 60);
        let lines: Vec<String> = blocks
            .window(0, height)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(!lines[0].trim().is_empty(), "{lines:?}");
        assert_eq!(lines.iter().filter(|l| l.trim().is_empty()).count(), 1);
    }

    /// A thought, once it is one.
    fn thought(id: &str, text: &str, seconds: i64) -> Item {
        let mut thought = item(
            id,
            ItemStatus::Completed,
            ItemBody::Reasoning {
                text: text.into(),
                provider_metadata: Default::default(),
            },
        );
        thought.completed_at = Some(ts() + jiff::SignedDuration::from_secs(seconds));
        // Outside any turn: a turn's thoughts fold under its line.
        thought.turn = None;
        thought
    }

    /// A run of thoughts draws as one block, hung on its last item (M79), so
    /// the thought that *was* last stops being a block the frame a newer one
    /// starts. `last` in the revision is what says so, and the rendering has to
    /// go with the entry: a stale block would stand under the run's own, with
    /// a blank row between them.
    #[test]
    fn a_thought_a_newer_one_joined_gives_its_block_back() {
        let mut state = state();
        state.transcript = vec![thought("itm_1", "The manifest first.", 2)]
            .into_iter()
            .collect();
        let mut blocks = cache();
        let alone = sync(&mut blocks, &state, 60);
        let top = 0;
        assert_eq!(blocks.at(top), Some(ItemId::from_raw("itm_1")));

        state.transcript.apply(&bingo_sdk::Event::ItemCompleted {
            item: thought("itm_2", "Then the crate map.", 3),
        });
        let joined = sync(&mut blocks, &state, 60);
        let lines: Vec<String> = blocks
            .window(0, joined)
            .iter()
            .map(|line| line.to_string().trim_end().to_string())
            .collect();
        assert_eq!(blocks.blocks.len(), 1, "one block for the run: {lines:?}");
        assert_eq!(joined, alone + 1, "one row more, and no blank one");
        assert_eq!(
            &lines[top..],
            [
                " ▏ Thought for 5s",
                "  ⎿  The manifest first.",
                "     Then the crate map.",
            ],
        );
        assert_eq!(
            blocks.at(top),
            Some(ItemId::from_raw("itm_2")),
            "the run's block, and its rows, are the last thought's"
        );
    }

    #[test]
    fn a_dropped_item_takes_its_block_with_it() {
        let mut state = many(4);
        let mut blocks = cache();
        sync(&mut blocks, &state, 60);
        state.transcript = state.transcript.items()[..2]
            .iter()
            .map(|item| item.as_ref().clone())
            .collect();
        sync(&mut blocks, &state, 60);
        assert_eq!(
            blocks.blocks.len(),
            2,
            "the cache is never larger than the transcript"
        );
    }

    #[test]
    fn a_failed_turn_keeps_its_line_at_the_foot_of_the_transcript() {
        let state = folded(vec![
            frame(
                1,
                Event::ItemCompleted {
                    item: user("itm_1", "go"),
                },
            )
            .clone(),
        ]);
        let mut with_failure = state.clone();
        with_failure.last_turn = Some(bingo_sdk::LastTurn {
            id: bingo_sdk::TurnId::from_raw("trn_1"),
            status: bingo_sdk::TurnStatus::Failed {
                error: bingo_sdk::KernelError::new(bingo_sdk::ErrorCode::Internal, "boom"),
            },
            started_at: ts(),
            ended_at: ts(),
            usage: Default::default(),
        });
        let mut blocks = cache();
        let plain = sync(&mut blocks, &state, 60);
        let failed = sync(&mut blocks, &with_failure, 60);
        assert_eq!(failed, plain + 2);
        assert!(blocks.tail(1)[0].contains("boom"), "{:?}", blocks.tail(1));
    }

    #[test]
    fn an_answer_has_no_surface_pacing_or_landing_deadline() {
        let mut state = state();
        let mut blocks = cache();
        let (_, now) = scene();
        for status in [ItemStatus::Running, ItemStatus::Completed] {
            state.transcript = vec![assistant("itm_1", "whole answer", status)]
                .into_iter()
                .collect();
            sync_at(&mut blocks, &state, 60, now);
            assert!(!blocks.moving(), "arrival is the only pacing");
            let rendered = blocks.renders();
            sync_at(&mut blocks, &state, 60, later(now, 5000));
            assert_eq!(blocks.renders(), rendered);
        }
    }

    #[test]
    fn the_tail_is_the_last_rows_as_plain_text() {
        let state = many(10);
        let mut blocks = cache();
        let height = sync(&mut blocks, &state, 60);
        let tail = blocks.tail(4);
        assert_eq!(tail.len(), 4);
        let whole = blocks.window(0, height);
        assert_eq!(tail[3], whole[height - 1].to_string().trim_end());
        assert!(tail[3].contains("Answer 9"), "{tail:?}");
    }
}

#[cfg(test)]
mod refresh_tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::Event;

    #[test]
    fn rowed_running_call_keeps_its_pulse_deadline() {
        crate::theme::with(crate::painted::daylight(), || {
            for replacement in [
                Some((true, false)),
                Some((true, true)),
                Some((false, true)),
                None,
            ] {
                let mut item = running_tool("itm_1", "Inspect", "checking");
                item.turn = None;
                let mut state = folded(vec![frame(1, Event::ItemStarted { item })]);
                if let Some((title, body)) = replacement {
                    state = crate::transcript::slots::fixtures::with_row(
                        state,
                        bingo_sdk::slot::Slot::Row {
                            item: ItemId::from_raw("itm_1"),
                            mark: Some('❖'),
                            title: title.then(|| "Review".into()),
                            badge: None,
                            caption: None,
                            body: body.then(|| bingo_sdk::View::text("replacement")),
                            actions: vec![],
                        },
                    );
                }
                let tree = solo(&state);
                let (mut ui, now) = scene();
                draw_tree(80, 24, &tree, &mut ui, now);
                assert!(
                    ui.shown.deadline().is_some(),
                    "replacement: {replacement:?}"
                );
                let before = ui.shown.blocks.blocks[0].lines[0]
                    .spans
                    .iter()
                    .find(|span| !span.content.trim().is_empty())
                    .unwrap()
                    .content
                    .to_string();
                draw_tree(80, 24, &tree, &mut ui, later(now, 500));
                let after = ui.shown.blocks.blocks[0].lines[0]
                    .spans
                    .iter()
                    .find(|span| !span.content.trim().is_empty())
                    .unwrap()
                    .content
                    .to_string();
                assert_ne!(before, after, "replacement: {replacement:?}");
                draw_tree(80, 24, &tree, &mut ui, still(later(now, 1000)));
                assert_eq!(
                    ui.shown.deadline(),
                    Some(crate::clock::next_tick(
                        later(now, 1000),
                        Duration::from_secs(1)
                    ))
                );
            }
        });
    }

    #[test]
    fn refresh_growth_repositions_the_inline_ask() {
        let state = folded(vec![
            frame(
                1,
                Event::ItemCompleted {
                    item: loose(assistant(
                        "itm_1",
                        "first line\n\nlast line",
                        ItemStatus::Completed,
                    )),
                },
            ),
            frame(
                2,
                Event::ItemStarted {
                    item: running_tool("itm_2", "Edit", "checking"),
                },
            ),
            frame(3, opened(permission(None, None))),
        ]);
        let tree = solo(&state);
        let (mut ui, now) = settled();
        ui.dialog.focus_on(state.interactions.first());
        draw_tree(80, 24, &tree, &mut ui, now);
        let expected = ui.shown.blocks.span(&ItemId::from_raw("itm_2")).unwrap();
        let first = &mut ui.shown.blocks.blocks[0];
        assert_eq!(first.id, ItemId::from_raw("itm_1"));
        assert!(first.lines.len() > 1);
        first.lines.pop().unwrap();
        first.next = Some(now.instant);
        ui.shown.blocks.height = ui.shown.blocks.measure();
        draw_tree(80, 24, &tree, &mut ui, later(now, 40));
        let (at, removed, _) = ui.shown.inline.unwrap();
        assert_eq!((at, removed), (expected.0, expected.1 - expected.0));
        let area = ui.shown.regions.transcript;
        let card = ui.shown.card.as_ref().unwrap();
        assert_eq!(
            usize::from(card.area.y - area.y) + ui.shown.top - ui.shown.padding(),
            at
        );
        let end = ui.shown.blocks.span(&ItemId::from_raw("itm_1")).unwrap().1;
        let above: String = (area.x..area.right())
            .map(|x| {
                ui.shown.canvas.buffer[(
                    x,
                    area.y + (end - ui.shown.top - 1 + ui.shown.padding()) as u16,
                )]
                    .symbol()
            })
            .collect();
        assert!(above.contains("last line"), "{above:?}");
        assert_eq!(
            ui.shown.line_at(at - ui.shown.top - 1 + ui.shown.padding()),
            Some(at - 1)
        );
    }

    #[test]
    fn refresh_can_remove_the_first_of_two_expiring_entries() {
        let mut state = folded(vec![
            frame(
                1,
                Event::ItemCompleted {
                    item: assistant("first", "vanishing", ItemStatus::Completed),
                },
            ),
            frame(
                2,
                Event::ItemCompleted {
                    item: assistant("second", "remaining", ItemStatus::Completed),
                },
            ),
        ]);
        let (mut ui, now) = scene();
        draw_tree(80, 24, &solo(&state), &mut ui, now);
        let blocks = &mut ui.shown.blocks;
        assert_eq!(blocks.blocks.len(), 2);
        for entry in &mut blocks.blocks {
            entry.next = Some(now.instant);
        }
        state.apply(&frame(
            3,
            Event::ItemCompleted {
                item: assistant("first", "", ItemStatus::Completed),
            },
        ));
        let folds = crate::fold::Folds::new();
        let pictures = crate::graphics::Decoded::default();
        let linked = crate::graphics::Linked::default();
        let rows = Rows::of(&state, 80, &folds, &pictures, &linked, now);
        let before = blocks.renders();
        blocks.refresh(&state, &Agents::new(), &rows, 0, blocks.height());
        assert_eq!(blocks.blocks.len(), 1);
        assert_eq!(blocks.blocks[0].id, ItemId::from_raw("second"));
        assert_eq!(blocks.renders(), before + 2);
    }

    #[test]
    fn refresh_growth_updates_height_and_followed_bottom() {
        let state = folded(
            (0..30)
                .map(|i| {
                    frame(
                        i + 1,
                        Event::ItemCompleted {
                            item: assistant(
                                &format!("item-{i}"),
                                "first line\n\nlast line",
                                ItemStatus::Completed,
                            ),
                        },
                    )
                })
                .collect(),
        );
        let (mut ui, now) = scene();
        let tree = solo(&state);
        draw_tree(80, 24, &tree, &mut ui, now);
        let expected_height = ui.shown.blocks.height();
        // Hold a shorter, expired drawing so refresh must restore its last row.
        let last = ui.shown.blocks.blocks.last_mut().unwrap();
        last.lines.pop().unwrap();
        last.next = Some(now.instant);
        ui.shown.blocks.height = ui.shown.blocks.measure();
        draw_tree(80, 24, &tree, &mut ui, later(now, 40));
        assert_eq!(ui.shown.blocks.height(), expected_height);
        assert_eq!(ui.shown.height, expected_height + 1);
        let area = ui.shown.regions.transcript;
        assert_eq!(ui.shown.top + usize::from(area.height), expected_height + 1);
        let bottom: String = (area.x..area.right())
            .map(|x| ui.shown.canvas.buffer[(x, area.bottom() - 2)].symbol())
            .collect();
        assert!(bottom.contains("last line"), "{bottom:?}");
        assert_eq!(
            ui.shown.line_at(usize::from(area.height) - 2),
            Some(expected_height - 1)
        );
    }
}
