//! The one binding table. The `?` panel and the footer hint both read it, so a
//! key can never be documented in one place and bound in another.

pub mod commands;

pub struct Binding {
    /// The chord, lowercase and `+`-joined; ` · ` separates alternatives.
    pub keys: &'static str,
    pub description: &'static str,
}

/// In the order the panel lists them.
pub const BINDINGS: &[Binding] = &[
    Binding {
        keys: "ctrl+f",
        description: "line search / browse",
    },
    Binding {
        keys: "/",
        description: "commands, in this list",
    },
    Binding {
        keys: "enter",
        description: "send · open focused block",
    },
    Binding {
        keys: "shift+enter",
        description: "newline (ctrl+j · alt+enter · \\ enter)",
    },
    Binding {
        // The stack of [`ESCAPES`], in the order it is obeyed; a test holds
        // the two together.
        keys: "esc",
        description: "sheet → card → dropdown → interrupt → clear → latest → back → rewind → arm rewind",
    },
    Binding {
        keys: "esc esc",
        description: "rewind to an earlier turn",
    },
    Binding {
        keys: "ctrl+c",
        description: "stop · clear · twice exits",
    },
    Binding {
        keys: "ctrl+d",
        description: "exit on an empty input",
    },
    Binding {
        keys: "up/down",
        description: "history at first/last input line",
    },
    Binding {
        keys: "down",
        description: "next history entry",
    },
    Binding {
        keys: "ctrl+a/e",
        description: "line start/end (home · end)",
    },
    Binding {
        keys: "alt+b/f",
        description: "move one word",
    },
    Binding {
        keys: "ctrl+w/u/k",
        description: "delete word · to start · to end",
    },
    Binding {
        keys: "pgup/pgdn",
        description: "scroll the transcript",
    },
    Binding {
        // One gesture on one row (M92): the sheet does not scroll and this
        // table already fills eighty by twenty-four, so a row added is a row
        // pushed off. The key and the drag draw the same run — cells of the
        // whole transcript, reaching what has scrolled away — and the page
        // says what holding it past an edge does.
        keys: "v · drag",
        description: "select; arrows/drag extend; y/ctrl+c/release copy",
    },
    Binding {
        keys: "ctrl+r",
        description: "sessions, in this list",
    },
    Binding {
        keys: "ctrl+t",
        description: "tasks band on and off",
    },
    Binding {
        // The stack of [`TABS`], in the order it is obeyed; a test holds the
        // two together.
        keys: "tab",
        description: "complete → queue → next row",
    },
    Binding {
        keys: "shift+tab",
        description: "cycle permission mode",
    },
    Binding {
        keys: "1-9 · y/a/n",
        description: "answer the open dialog",
    },
    Binding {
        keys: "ctrl+e",
        description: "expand preview",
    },
    Binding {
        keys: "alt+p",
        description: "cycle pane place",
    },
    Binding {
        keys: "ctrl+o",
        description: "expand result · then all",
    },
    Binding {
        keys: "ctrl+b",
        description: "background running command",
    },
    Binding {
        // Mouse reporting has to reach us for this to happen at all: inside
        // tmux it wants `set -g mouse on`, and without it a click is the
        // multiplexer's own (M56).
        keys: "click",
        description: "a picture · open it",
    },
    Binding {
        keys: "?",
        description: "keys, in this list",
    },
];

// ---- the three ordered stacks (design §7) -------------------------------

/// The place both closing gestures leave, selected by the key policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Close {
    Sheet,
    Card,
    Dropdown,
    Pane,
    Search,
    Back,
}

/// What `esc` closes, innermost first. It is a table rather than a chain of
/// conditions so that the order is one thing, tested once and printed in the
/// help exactly as it is obeyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    /// Leave the selected place with its own close or cancel action.
    Close(Close),
    /// Nothing is open, so the turn is what `esc` stops.
    Interrupt,
    /// Save the draft in history and empty the input.
    Clear,
    /// Return the transcript to its newest row.
    Latest,
    /// The second empty escape opens the rewind picker.
    Rewind,
    /// The first empty escape says how to rewind.
    Arm,
}

/// The stack, in the order it is closed, with the word the help prints.
pub const ESCAPES: &[(Escape, &str)] = &[
    (Escape::Close(Close::Sheet), "sheet"),
    (Escape::Close(Close::Card), "card"),
    (Escape::Close(Close::Dropdown), "dropdown"),
    (Escape::Interrupt, "interrupt"),
    (Escape::Clear, "clear"),
    (Escape::Latest, "latest"),
    (Escape::Close(Close::Back), "back"),
    (Escape::Rewind, "rewind"),
    (Escape::Arm, "arm rewind"),
];

/// What is open when `esc` is pressed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Open {
    pub sheet: bool,
    pub card: bool,
    pub dropdown: bool,
    pub pane: bool,
    pub search: bool,
    pub busy: bool,
    pub subsession: bool,
    pub typing: bool,
    pub scrolled: bool,
    pub rewind: bool,
    pub armed: bool,
}

impl Open {
    fn has(self, rung: Escape) -> bool {
        match rung {
            Escape::Close(Close::Sheet) => self.sheet,
            Escape::Close(Close::Card) => self.card,
            Escape::Close(Close::Dropdown) => self.dropdown,
            Escape::Close(Close::Pane) => self.pane,
            Escape::Close(Close::Search) => self.search,
            Escape::Interrupt => self.busy && !self.subsession,
            Escape::Clear => self.typing,
            Escape::Latest => self.scrolled,
            Escape::Rewind => self.rewind && self.armed,
            Escape::Arm => self.rewind,
            Escape::Close(Close::Back) => self.subsession,
        }
    }
}

/// The innermost thing `esc` closes, and nothing at all when a person has
/// nothing open and nothing running.
pub fn escape(open: Open) -> Option<Escape> {
    // Pane and search owners consume Esc before the composer's stack.
    [Escape::Close(Close::Pane), Escape::Close(Close::Search)]
        .into_iter()
        .chain(ESCAPES.iter().map(|(rung, _)| *rung))
        .find(|rung| open.has(*rung))
}

/// What `tab` does, innermost first (§7). A layer that captures never reaches
/// this table — the switcher and a form answer their own keys while
/// they are up — so every rung here is about the composer and the cards
/// beneath it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// The dropdown is open: take the row the cursor is on.
    Complete,
    /// A turn is running and there are words in the box: the line waits for
    /// that turn to end instead of steering it (ADR-0008 §2, amended M68).
    Queue,
    /// Move the ring to the next row that answers keys.
    Focus,
}

/// The stack, in the order it is obeyed, with the word the help prints.
pub const TABS: &[(Tab, &str)] = &[
    (Tab::Complete, "complete"),
    (Tab::Queue, "queue"),
    (Tab::Focus, "next row"),
];

/// What is true when `tab` is pressed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tabbed {
    pub dropdown: bool,
    pub busy: bool,
    /// There are words in the box: `tab` on an empty one queues nothing.
    pub typing: bool,
    pub focusable: bool,
}

impl Tabbed {
    fn has(self, rung: Tab) -> bool {
        match rung {
            Tab::Complete => self.dropdown,
            Tab::Queue => self.busy && self.typing,
            Tab::Focus => self.focusable,
        }
    }
}

/// The innermost thing `tab` does, and nothing at all when there is nothing to
/// complete, nothing to queue and no card to walk to.
pub fn tab(at: Tabbed) -> Option<Tab> {
    TABS.iter()
        .map(|(rung, _)| *rung)
        .find(|rung| at.has(*rung))
}

/// What `ctrl+c` does, which is not what `esc` does: it is the key that gets
/// you out, and it says so before it takes you (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interrupt {
    /// A turn is running: stop it, and leave the composer alone.
    Turn,
    Close(Close),
    Fold,
    /// Something is half-typed: clear it.
    Clear,
    /// Nothing to stop and nothing to clear: say how to leave.
    Arm,
    /// It was said a moment ago: leave.
    Exit,
}

/// What is true when `ctrl+c` is pressed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pressed {
    pub open: Open,
    pub detail: bool,
    /// A first `ctrl+c` inside the window said how to leave.
    pub armed: bool,
}

pub const ESC_WINDOW: std::time::Duration = std::time::Duration::from_millis(600);
pub const CLEARED_HOLD: std::time::Duration = std::time::Duration::from_millis(1800);
pub const EXIT_WINDOW: std::time::Duration = std::time::Duration::from_millis(1500);

pub fn twice(
    first: Option<std::time::Instant>,
    now: std::time::Instant,
    window: std::time::Duration,
) -> bool {
    first.is_some_and(|first| now.saturating_duration_since(first) < window)
}

pub fn interrupt(at: Pressed) -> Interrupt {
    let close = match escape(at.open) {
        Some(Escape::Close(place)) => Some(Interrupt::Close(place)),
        _ => None,
    };
    [
        at.open.busy.then_some(Interrupt::Turn),
        close,
        at.detail.then_some(Interrupt::Fold),
        at.open.typing.then_some(Interrupt::Clear),
        at.armed.then_some(Interrupt::Exit),
        Some(Interrupt::Arm),
    ]
    .into_iter()
    .flatten()
    .next()
    .unwrap_or(Interrupt::Arm)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn every_acting_rung_obeys_the_policy_order() {
        let all = Open {
            typing: true,
            scrolled: true,
            rewind: true,
            armed: true,
            ..Open::default()
        };
        assert_eq!(escape(all), Some(Escape::Clear));
        assert_eq!(
            escape(Open {
                typing: false,
                ..all
            }),
            Some(Escape::Latest)
        );
        assert_eq!(
            escape(Open {
                typing: false,
                scrolled: false,
                ..all
            }),
            Some(Escape::Rewind)
        );
        assert_eq!(
            escape(Open {
                typing: false,
                scrolled: false,
                armed: false,
                ..all
            }),
            Some(Escape::Arm)
        );
        assert_eq!(
            escape(Open {
                busy: true,
                subsession: true,
                ..Open::default()
            }),
            Some(Escape::Close(Close::Back))
        );
        let mut pressed = Pressed {
            open: Open {
                busy: true,
                sheet: true,
                typing: true,
                ..Open::default()
            },
            detail: true,
            armed: true,
        };
        assert_eq!(interrupt(pressed), Interrupt::Turn);
        pressed.open.busy = false;
        assert_eq!(interrupt(pressed), Interrupt::Close(Close::Sheet));
        pressed.open.sheet = false;
        assert_eq!(interrupt(pressed), Interrupt::Fold);
        pressed.detail = false;
        assert_eq!(interrupt(pressed), Interrupt::Clear);
        pressed.open.typing = false;
        assert_eq!(interrupt(pressed), Interrupt::Exit);
        pressed.armed = false;
        assert_eq!(interrupt(pressed), Interrupt::Arm);
    }

    #[test]
    fn every_chord_is_listed_once() {
        let mut seen: Vec<&str> = BINDINGS.iter().map(|b| b.keys).collect();
        seen.sort_unstable();
        let unique = {
            let mut u = seen.clone();
            u.dedup();
            u
        };
        assert_eq!(seen, unique, "one key, one row");
    }

    /// One test per rung: what is innermost is what closes.
    #[test]
    fn esc_closes_the_innermost_thing_that_is_open() {
        let all = Open {
            sheet: true,
            card: true,
            dropdown: true,
            busy: true,
            subsession: true,
            ..Open::default()
        };
        assert_eq!(escape(all), Some(Escape::Close(Close::Sheet)));
        assert_eq!(
            escape(Open {
                sheet: false,
                ..all
            }),
            Some(Escape::Close(Close::Card))
        );
        assert_eq!(
            escape(Open {
                sheet: false,
                card: false,
                ..all
            }),
            Some(Escape::Close(Close::Dropdown))
        );
        assert_eq!(
            escape(Open {
                busy: true,
                ..Open::default()
            }),
            Some(Escape::Interrupt)
        );
        assert_eq!(
            escape(Open {
                subsession: true,
                ..Open::default()
            }),
            Some(Escape::Close(Close::Back))
        );
        assert_eq!(escape(Open::default()), None, "and nothing to close");
    }

    /// The order `tab` is obeyed in is the order the help prints, because
    /// there is only one of it.
    #[test]
    fn the_help_prints_the_tab_stack() {
        let stack = TABS
            .iter()
            .map(|(_, word)| *word)
            .collect::<Vec<_>>()
            .join(" → ");
        let tab = BINDINGS
            .iter()
            .find(|binding| binding.keys == "tab")
            .expect("a row for tab");
        assert_eq!(tab.description, stack);
    }

    /// The three cases the risk register named: the dropdown still completes,
    /// a running turn with words in the box queues them, and a card ring is
    /// what is left. `tab` on an empty box queues nothing.
    #[test]
    fn tab_completes_then_queues_then_walks_the_cards() {
        let all = Tabbed {
            dropdown: true,
            busy: true,
            typing: true,
            focusable: true,
        };
        assert_eq!(tab(all), Some(Tab::Complete));
        assert_eq!(
            tab(Tabbed {
                dropdown: false,
                ..all
            }),
            Some(Tab::Queue)
        );
        assert_eq!(
            tab(Tabbed {
                dropdown: false,
                typing: false,
                ..all
            }),
            Some(Tab::Focus),
            "an empty box queues nothing, so the ring has it"
        );
        assert_eq!(
            tab(Tabbed {
                dropdown: false,
                busy: false,
                ..all
            }),
            Some(Tab::Focus),
            "and with no turn running there is nothing to wait for"
        );
        assert_eq!(tab(Tabbed::default()), None);
    }

    /// One test per row of what ctrl+c does.
    #[test]
    fn ctrl_c_stops_a_turn_clears_a_line_and_then_leaves() {
        let pressed = |busy, typing, armed| {
            interrupt(Pressed {
                open: Open {
                    busy,
                    typing,
                    ..Open::default()
                },
                armed,
                ..Pressed::default()
            })
        };
        assert_eq!(pressed(true, true, true), Interrupt::Turn);
        assert_eq!(pressed(false, true, true), Interrupt::Clear);
        assert_eq!(pressed(false, false, true), Interrupt::Exit);
        assert_eq!(pressed(false, false, false), Interrupt::Arm);
    }
}
