//! What this terminal can draw beside the words.
//!
//! Design §5's image row: a picture is pixels where the terminal speaks the
//! kitty graphics protocol, and `[image: …]` everywhere else. Which of the two
//! is *asked*, once, at start-up — never assumed from `TERM`, which says what
//! a terminal calls itself and not what it can do — the way the background
//! colour is asked (`theme.rs`).
//!
//! - [`probe`] is the questions and the reading of the answers, both pure.
//! - [`placeholders`] is the list of terminals that draw a placeholder cell,
//!   which is the one thing no terminal can be asked.
//! - [`unseen`] is the setting a herdr pane that draws chips is told.
//! - [`tmux`] is the multiplexer the questions and the pictures travel
//!   through when there is one, and the envelope they travel in.
//! - [`picture`] is one picture a frame drew: where it came from and how many
//!   cells it took.
//! - [`band`] is a few of them side by side, small enough to glance at — the
//!   one shape the composer's strip and a person's own `>` block both wear.
//! - [`kitty`] is the protocol as bytes.
//! - [`placed`] reads back out of a drawn frame where each picture's cells
//!   landed, which is what a click on one is answered against.
//! - [`linked`] keeps the pictures the words themselves named, read in once.
//! - [`decoded`] keeps the pixels, so a picture is decoded once.
//! - [`stored`] keeps what the terminal is holding, and says what to send.
//!
//! An answer slower than the probe's clock lands in crossterm's key stream
//! instead of in the probe's read. [`crate::late`] hears it there, and [`late`]
//! joins it to what was heard in time (M60).

pub mod band;
pub mod decoded;
pub mod kitty;
pub mod linked;
pub mod picture;
pub mod placed;
pub mod placeholders;
pub mod probe;
pub mod stored;
pub mod tmux;
pub mod unseen;

pub use band::Band;
pub use decoded::{Decoded, Pixels};
pub use linked::Linked;
pub use picture::Picture;
pub use placed::Placed;
pub use placeholders::draws_placeholders;
pub use probe::Probe;
pub use stored::Stored;
pub use tmux::{Passthrough, Transport};

/// One cell of this terminal, in pixels. What turns a picture's size into a
/// number of cells, and the reason a terminal that will not say draws no
/// pictures: a guess of 8×16 would draw every picture the wrong shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub width: u16,
    pub height: u16,
}

/// How this run draws a picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Graphics {
    /// The chip of design §5, which every terminal can draw.
    #[default]
    Off,
    Kitty {
        cell: Cell,
        /// What the bytes of a picture have to travel in to reach the
        /// terminal that draws it.
        transport: Transport,
    },
}

impl Graphics {
    /// All of it or none. A terminal that speaks the protocol but will not
    /// say how big a cell is cannot be drawn into; one that says `OK` and is
    /// not known to draw a placeholder cell would draw tofu where the picture
    /// goes, and a stray copy of it at the cursor besides
    /// ([`draws_placeholders`]), so it gets the chip.
    ///
    /// Under tmux there is a second name to satisfy: a tmux old enough to
    /// mangle the placeholder cells is no route at all, however good the
    /// terminal behind it ([`tmux::carries_pictures`]).
    pub fn from(probe: &Probe, transport: Transport) -> Self {
        match (reachable(probe, transport), probe.cell) {
            (true, Some(cell)) => Graphics::Kitty { cell, transport },
            _ => Graphics::Off,
        }
    }

    /// What the bytes of a picture travel in. A run that draws none has none
    /// to send, and answers with the transport it never uses.
    pub fn transport(self) -> Transport {
        match self {
            Graphics::Off => Transport::Bare,
            Graphics::Kitty { transport, .. } => transport,
        }
    }
}

/// Whether a picture put on the wire reaches a terminal that draws it.
fn reachable(probe: &Probe, transport: Transport) -> bool {
    if !probe.kitty || !probe.terminals.iter().any(draws_placeholders) {
        return false;
    }
    match transport {
        Transport::Bare => true,
        Transport::Tmux => probe.terminals.iter().any(tmux::carries_pictures),
    }
}

/// What tmux has to be told when the setting that carries a picture is the one
/// that is off. tmux said so itself, so the notice names the one thing to
/// change and nothing else (M60 brick 3).
pub const PASSTHROUGH_OFF: &str = "tmux: pictures need `set -g allow-passthrough on`";

/// What tmux has to be told when the passthrough is on — or tmux would not
/// say — and nothing came out of it all the same: this pane was not the
/// focused one when bingo started, and the outer terminal's answers went to
/// whichever pane was (tmux delivers them to the active pane, M49 risk 1).
pub const PASSTHROUGH_UNHEARD: &str = "tmux: pictures need bingo started in the focused pane";

/// Whether nothing behind tmux answered: tmux's own name, or no name at all
/// when the question was never sent.
fn only_tmux(probe: &Probe) -> bool {
    !probe.kitty && probe.cell.is_none() && probe.terminals.iter().all(tmux::named)
}

/// The one thing there is to say about a terminal that draws no pictures:
/// what tmux has to be told, or else the setting a herdr pane needs
/// ([`unseen`]). Every other silence is the terminal's own and has nothing
/// to tell.
fn notice(heard: &Heard) -> Option<&'static str> {
    unheard(heard).or_else(|| unseen::unseen(heard))
}

/// What tmux has to be told when nothing behind it was heard.
fn unheard(heard: &Heard) -> Option<&'static str> {
    if heard.transport != Transport::Tmux || !only_tmux(&heard.probe) {
        return None;
    }
    match heard.passthrough {
        // No envelope was sent, so of course nothing came out of one.
        Passthrough::Off => Some(PASSTHROUGH_OFF),
        // A tmux that did not answer for itself either is a probe that
        // failed, and a failed probe has nothing to tell anybody.
        Passthrough::On | Passthrough::All | Passthrough::Unknown => {
            (!heard.probe.terminals.is_empty()).then_some(PASSTHROUGH_UNHEARD)
        }
    }
}

/// What the terminal said, and what its words had to travel through to be
/// heard: the one fact this module keeps. How the run draws pictures and what
/// it has to say about that are both read out of it ([`Settled`]), so neither
/// can go stale against the other or against the answer they came from.
///
/// There is no empty one: a run that asked nothing has no `Heard` at all, so
/// "never asked" cannot be mistaken for "asked and heard nothing" — and a
/// reply that turns up late for a question nobody put has nothing to join.
#[derive(Clone, Debug)]
pub(crate) struct Heard {
    pub probe: Probe,
    pub transport: Transport,
    pub passthrough: Passthrough,
}

impl Heard {
    /// A reply that arrived after the read ended, joined to what was heard in
    /// time. Monotone ([`Probe::and`]), which is what makes one late settle
    /// happen and no second one: a reply that says nothing new settles
    /// nothing.
    pub fn and(&mut self, later: Probe) {
        self.probe = std::mem::take(&mut self.probe).and(later);
    }
}

/// What the probe settled: how this run draws pictures, and the one thing
/// there is to say about it. Both come from the one answer, so neither can
/// go stale against the other.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Settled {
    pub graphics: Graphics,
    pub notice: Option<&'static str>,
}

impl Settled {
    fn of(heard: &Heard) -> Self {
        Settled {
            notice: notice(heard),
            graphics: Graphics::from(&heard.probe, heard.transport),
        }
    }
}

/// Whether this run draws pictures at all: `BINGO_GRAPHICS=off` says no, as
/// `BINGO_MOTION=off` and `BINGO_ASCII=1` say their own noes.
pub fn wanted(setting: Option<&str>) -> bool {
    setting != Some("off")
}

/// Pure detection from the answers belonging to this run.
pub(crate) fn detect(heard: Option<&Heard>) -> Settled {
    heard.map(Settled::of).unwrap_or_default()
}

pub fn chosen() -> Graphics {
    crate::render::current().graphics
}

#[cfg(test)]
pub fn with<R>(graphics: Graphics, f: impl FnOnce() -> R) -> R {
    crate::render::Render {
        graphics,
        ..crate::render::current()
    }
    .scope(f)
}

/// A terminal that draws pictures, with the cell most of them have.
#[cfg(test)]
pub fn drawing() -> Graphics {
    Graphics::Kitty {
        cell: Cell {
            width: 10,
            height: 20,
        },
        transport: Transport::Bare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use probe::Named;

    const CELL: Cell = Cell {
        width: 10,
        height: 20,
    };

    fn named(names: &[(&str, &str)]) -> Vec<Named> {
        names
            .iter()
            .map(|(name, version)| Named {
                name: (*name).into(),
                version: (*version).into(),
            })
            .collect()
    }

    fn answered(kitty: bool, cell: Option<Cell>, terminals: &[(&str, &str)]) -> Probe {
        Probe {
            kitty,
            cell,
            terminals: named(terminals),
        }
    }

    /// One terminal, nothing in the way.
    fn bare(kitty: bool, cell: Option<Cell>, terminals: &[(&str, &str)]) -> Graphics {
        Graphics::from(&answered(kitty, cell, terminals), Transport::Bare)
    }

    /// The same answers, through tmux.
    fn through_tmux(kitty: bool, cell: Option<Cell>, terminals: &[(&str, &str)]) -> Graphics {
        Graphics::from(&answered(kitty, cell, terminals), Transport::Tmux)
    }

    /// One answer, and everything it had to travel through to be heard.
    fn heard(probe: Probe, transport: Transport, passthrough: Passthrough) -> Heard {
        Heard {
            probe,
            transport,
            passthrough,
        }
    }

    const KITTY: (&str, &str) = ("kitty", "0.46.2");
    const GHOSTTY: (&str, &str) = ("ghostty", "1.3.1");
    const TMUX: (&str, &str) = ("tmux", "3.6b");

    #[test]
    fn pictures_are_drawn_only_when_every_part_of_the_answer_came_back() {
        assert_eq!(
            bare(true, Some(CELL), &[KITTY]),
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Bare
            }
        );
        assert_eq!(
            bare(true, None, &[KITTY]),
            Graphics::Off,
            "no cell size is no picture, rather than a guessed one"
        );
        assert_eq!(bare(false, Some(CELL), &[KITTY]), Graphics::Off);
    }

    /// The terminals of the M48 list, through the whole answer: the two that
    /// draw the cells, and the three that say `OK` and would draw tofu.
    #[test]
    fn a_terminal_that_says_ok_and_draws_no_placeholder_gets_the_chip() {
        for terminal in [KITTY, GHOSTTY] {
            assert_eq!(
                bare(true, Some(CELL), &[terminal]),
                Graphics::Kitty {
                    cell: CELL,
                    transport: Transport::Bare
                },
                "{terminal:?}"
            );
        }
        for terminal in [
            ("WezTerm", "20240203-110809-5046fc22"),
            ("Konsole", "26.08.0"),
            ("kitty", "0.27.9"),
        ] {
            assert_eq!(
                bare(true, Some(CELL), &[terminal]),
                Graphics::Off,
                "{terminal:?}"
            );
        }
    }

    /// herdr's pane: a bare `libghostty` draws pictures once there is a cell
    /// to draw them into, and the chip while there is none.
    #[test]
    fn a_herdr_pane_draws_pictures_only_with_a_cell() {
        let herdr = ("libghostty", "");
        assert_eq!(
            bare(true, Some(CELL), &[herdr]),
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Bare
            }
        );
        assert_eq!(bare(true, None, &[herdr]), Graphics::Off);
    }

    /// A terminal that says `OK` and will not say what it is says nothing
    /// about placeholders either, and the chip is what silence gets.
    #[test]
    fn a_terminal_that_names_itself_to_nobody_gets_the_chip() {
        assert_eq!(bare(true, Some(CELL), &[]), Graphics::Off);
    }

    /// M49 brick 2: through tmux the whole answer has one more name in it,
    /// and the pictures go out in tmux's envelope.
    #[test]
    fn a_ghostty_behind_a_new_enough_tmux_draws_pictures() {
        assert_eq!(
            through_tmux(true, Some(CELL), &[TMUX, GHOSTTY]),
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Tmux
            }
        );
    }

    /// A tmux that mangles the placeholder cells is no route, whatever is
    /// behind it; nor is a tmux that never said which one it is.
    #[test]
    fn a_tmux_below_the_floor_draws_no_pictures() {
        assert_eq!(
            through_tmux(true, Some(CELL), &[("tmux", "3.3a"), KITTY]),
            Graphics::Off
        );
        assert_eq!(
            through_tmux(true, Some(CELL), &[KITTY]),
            Graphics::Off,
            "the envelope's own carrier never named itself"
        );
    }

    /// And an outer terminal off the list is off it under tmux too.
    #[test]
    fn a_wezterm_behind_tmux_draws_no_pictures() {
        assert_eq!(
            through_tmux(
                true,
                Some(CELL),
                &[TMUX, ("WezTerm", "20240203-110809-5046fc22")]
            ),
            Graphics::Off
        );
    }

    /// M49 brick 3, reworded by M60 brick 3: tmux answered and nothing behind
    /// it did, so the passthrough never carried the question. That is the one
    /// silence worth a word — and the only one that gets one.
    #[test]
    fn only_a_tmux_nobody_answered_behind_is_told_about() {
        let alone = answered(false, None, &[TMUX]);
        assert_eq!(
            unheard(&heard(alone.clone(), Transport::Tmux, Passthrough::On)),
            Some(PASSTHROUGH_UNHEARD)
        );
        assert_eq!(
            unheard(&heard(alone.clone(), Transport::Bare, Passthrough::On)),
            None,
            "with no tmux there is no passthrough to ask for"
        );
        assert_eq!(
            unheard(&heard(
                answered(true, Some(CELL), &[TMUX, GHOSTTY]),
                Transport::Tmux,
                Passthrough::On
            )),
            None,
            "the outer terminal answered"
        );
        assert_eq!(
            unheard(&heard(
                answered(false, None, &[TMUX, ("foot", "1.28.0")]),
                Transport::Tmux,
                Passthrough::On
            )),
            None,
            "and so did one with no pictures in it"
        );
        assert_eq!(
            unheard(&heard(
                answered(false, None, &[]),
                Transport::Tmux,
                Passthrough::Unknown
            )),
            None,
            "a tmux that answered nothing itself is a probe that failed"
        );
    }

    /// M60 brick 3: tmux said the setting is off, so nothing was ever asked
    /// and the notice names the setting rather than the pane.
    #[test]
    fn a_passthrough_tmux_says_is_off_is_told_about_by_name() {
        assert_eq!(
            unheard(&heard(Probe::default(), Transport::Tmux, Passthrough::Off)),
            Some(PASSTHROUGH_OFF),
            "no envelope was sent, so there is no name in the answer at all"
        );
        assert_eq!(
            unheard(&heard(
                answered(true, Some(CELL), &[TMUX, GHOSTTY]),
                Transport::Tmux,
                Passthrough::Off
            )),
            None,
            "a setting that says off against an answer that came back is no \
             reason to tell anybody anything"
        );
    }

    /// Both halves of one answer, so neither can be right while the other is
    /// wrong: a passthrough that carried nothing draws no pictures *and*
    /// says why.
    #[test]
    fn what_the_probe_settles_is_the_drawing_and_the_word_about_it() {
        let off = Settled::of(&heard(
            answered(false, None, &[TMUX]),
            Transport::Tmux,
            Passthrough::On,
        ));
        assert_eq!(off.graphics, Graphics::Off);
        assert_eq!(off.notice, Some(PASSTHROUGH_UNHEARD));

        let on = Settled::of(&heard(
            answered(true, Some(CELL), &[TMUX, GHOSTTY]),
            Transport::Tmux,
            Passthrough::On,
        ));
        assert_eq!(
            on.graphics,
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Tmux
            }
        );
        assert_eq!(on.notice, None);
    }

    /// M60 brick 2: the answer the read gave up on, completed by what came
    /// late. The pictures come on and the notice that said they would not is
    /// no longer true — and merging again says nothing new, so nothing
    /// settles twice.
    #[test]
    fn an_answer_completed_late_turns_the_pictures_on_once() {
        let mut waiting = heard(
            answered(true, Some(CELL), &[TMUX]),
            Transport::Tmux,
            Passthrough::On,
        );
        assert_eq!(Settled::of(&waiting).graphics, Graphics::Off);

        waiting.and(answered(false, None, &[GHOSTTY]));
        let settled = Settled::of(&waiting);
        assert_eq!(
            settled.graphics,
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Tmux
            }
        );
        assert_eq!(settled.notice, None);

        waiting.and(answered(false, None, &[GHOSTTY]));
        assert_eq!(
            Settled::of(&waiting).graphics,
            settled.graphics,
            "the same reply twice is the same answer once"
        );
        assert_eq!(waiting.probe.terminals.len(), 2);
    }

    /// And a late reply that carries no cell — which is every late reply,
    /// because crossterm drops the cell reply without an event
    /// ([`crate::late`]) — takes the notice back without turning the
    /// pictures on: the terminal did answer, and saying it did not was wrong.
    #[test]
    fn a_late_answer_with_no_cell_still_takes_the_wrong_word_back() {
        let mut waiting = heard(
            answered(false, None, &[TMUX]),
            Transport::Tmux,
            Passthrough::On,
        );
        assert_eq!(Settled::of(&waiting).notice, Some(PASSTHROUGH_UNHEARD));
        waiting.and(answered(true, None, &[GHOSTTY]));
        let settled = Settled::of(&waiting);
        assert_eq!(settled.notice, None);
        assert_eq!(settled.graphics, Graphics::Off, "no cell is no picture");
    }

    /// herdr with pictures off: the chip, and the one line that says which
    /// setting turns them on. With them on there is nothing to say.
    #[test]
    fn a_herdr_pane_with_no_cell_draws_chips_and_names_the_setting() {
        let herdr = ("libghostty", "");
        let off = Settled::of(&heard(
            answered(true, None, &[herdr]),
            Transport::Bare,
            Passthrough::Unknown,
        ));
        assert_eq!(off.graphics, Graphics::Off);
        assert_eq!(off.notice, Some(unseen::HERDR_OFF));
        let on = Settled::of(&heard(
            answered(true, Some(CELL), &[herdr]),
            Transport::Bare,
            Passthrough::Unknown,
        ));
        assert_eq!(on.notice, None);
    }

    #[test]
    fn a_run_that_draws_nothing_has_no_transport_to_send_it_on() {
        assert_eq!(Graphics::Off.transport(), Transport::Bare);
        assert_eq!(
            Graphics::Kitty {
                cell: CELL,
                transport: Transport::Tmux
            }
            .transport(),
            Transport::Tmux
        );
    }

    #[test]
    fn only_off_switches_the_pictures_off() {
        assert!(!wanted(Some("off")));
        assert!(wanted(None));
        assert!(wanted(Some("")));
        assert!(wanted(Some("1")));
    }
}
