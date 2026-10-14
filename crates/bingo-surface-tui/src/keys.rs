//! The one binding table. The `?` panel and the footer hint both read it, so a
//! key can never be documented in one place and bound in another.

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme;

pub struct Binding {
    /// The chord, lowercase and `+`-joined; ` · ` separates alternatives.
    pub keys: &'static str,
    pub description: &'static str,
    pub group: &'static str,
}

/// In the order the panel lists them.
pub const BINDINGS: &[Binding] = &[
    Binding {
        keys: "enter",
        group: "write",
        description: "send · open focused block",
    },
    Binding {
        keys: "shift+enter",
        group: "write",
        description: "newline (ctrl+j · alt+enter · \\ enter)",
    },
    Binding {
        // The stack of [`ESCAPES`], in the order it is obeyed; a test holds
        // the two together.
        keys: "esc",
        group: "turn",
        description: "sheet → card → dropdown → interrupt",
    },
    Binding {
        keys: "esc esc",
        group: "turn",
        description: "rewind to an earlier turn",
    },
    Binding {
        keys: "ctrl+c",
        group: "turn",
        description: "stop · clear · twice exits",
    },
    Binding {
        keys: "ctrl+d",
        group: "turn",
        description: "exit on an empty input",
    },
    Binding {
        keys: "up/down",
        group: "move",
        description: "history at first/last input line",
    },
    Binding {
        keys: "down",
        group: "sessions",
        description: "sessions (empty input)",
    },
    Binding {
        keys: "ctrl+a/e",
        group: "write",
        description: "line start/end (home · end)",
    },
    Binding {
        keys: "alt+b/f",
        group: "write",
        description: "move one word",
    },
    Binding {
        keys: "ctrl+w/u/k",
        group: "write",
        description: "delete word · to start · to end",
    },
    Binding {
        keys: "pgup/pgdn",
        group: "move",
        description: "scroll the transcript",
    },
    Binding {
        // One gesture on one row (M92): the sheet does not scroll and this
        // table already fills eighty by twenty-four, so a row added is a row
        // pushed off. The key and the drag draw the same run — cells of the
        // whole transcript, reaching what has scrolled away — and the page
        // says what holding it past an edge does.
        keys: "v · drag",
        group: "move",
        description: "select; arrows/drag extend; y/ctrl+c/release copy",
    },
    Binding {
        keys: "ctrl+g",
        group: "sessions",
        description: "sessions · ↑↓ to walk",
    },
    Binding {
        keys: "ctrl+t",
        group: "turn",
        description: "show/hide tasks",
    },
    Binding {
        // The stack of [`TABS`], in the order it is obeyed; a test holds the
        // two together.
        keys: "tab",
        group: "turn",
        description: "complete → queue → next row",
    },
    Binding {
        keys: "shift+tab",
        group: "turn",
        description: "cycle permission mode",
    },
    Binding {
        keys: "1-9 · y/a/n",
        group: "turn",
        description: "answer the open dialog",
    },
    Binding {
        keys: "ctrl+e",
        group: "turn",
        description: "expand preview",
    },
    Binding {
        keys: "alt+p",
        group: "move",
        description: "cycle pane place",
    },
    Binding {
        keys: "ctrl+o",
        group: "move",
        description: "expand result · then all",
    },
    Binding {
        keys: "ctrl+b",
        group: "turn",
        description: "background running command",
    },
    Binding {
        // Mouse reporting has to reach us for this to happen at all: inside
        // tmux it wants `set -g mouse on`, and without it a click is the
        // multiplexer's own (M56).
        keys: "click",
        group: "move",
        description: "a picture · open it",
    },
    Binding {
        keys: "?",
        group: "sessions",
        description: "toggle this panel",
    },
];

pub const FOOTER_HINT: &str = "? for shortcuts";
pub const PLACEHOLDER: &str = "ask anything · / for commands · ! for shell";
/// What the empty box offers while a turn runs: the two ways a line goes out
/// of it (ADR-0008 §2, amended M68). One string, so the box and the help can
/// never say different things about the same key.
pub const BUSY_PLACEHOLDER: &str = "ask anything · ⏎ steers · tab queues";

// ---- the three ordered stacks (design §7) -------------------------------

/// What `esc` closes, innermost first. It is a table rather than a chain of
/// conditions so that the order is one thing, tested once and printed in the
/// help exactly as it is obeyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    /// A sheet over the whole frame: help, the resume picker.
    Sheet,
    /// The card that is asking: leaving it is its own cancel or denial.
    Card,
    /// The command dropdown.
    Dropdown,
    /// Nothing is open, so the turn is what `esc` stops.
    Interrupt,
}

/// The stack, in the order it is closed, with the word the help prints.
pub const ESCAPES: &[(Escape, &str)] = &[
    (Escape::Sheet, "sheet"),
    (Escape::Card, "card"),
    (Escape::Dropdown, "dropdown"),
    (Escape::Interrupt, "interrupt"),
];

/// What is open when `esc` is pressed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Open {
    pub sheet: bool,
    pub card: bool,
    pub dropdown: bool,
    pub busy: bool,
}

impl Open {
    fn has(self, rung: Escape) -> bool {
        match rung {
            Escape::Sheet => self.sheet,
            Escape::Card => self.card,
            Escape::Dropdown => self.dropdown,
            Escape::Interrupt => self.busy,
        }
    }
}

/// The innermost thing `esc` closes, and nothing at all when a person has
/// nothing open and nothing running.
pub fn escape(open: Open) -> Option<Escape> {
    ESCAPES
        .iter()
        .map(|(rung, _)| *rung)
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
    pub busy: bool,
    pub typing: bool,
    /// A first `ctrl+c` inside the window said how to leave.
    pub armed: bool,
}

pub fn interrupt(at: Pressed) -> Interrupt {
    match (at.busy, at.typing, at.armed) {
        (true, _, _) => Interrupt::Turn,
        (false, true, _) => Interrupt::Clear,
        (false, false, true) => Interrupt::Exit,
        (false, false, false) => Interrupt::Arm,
    }
}

/// From this width the table pairs its rows; below it one column reads better
/// than two cut-off ones.
pub const TWO_COLUMNS: usize = 80;
/// Cells between the two columns.
const GUTTER: usize = 2;

/// Pair groups on a wide screen; wrap words instead of clipping bindings.
pub fn help_lines(width: usize) -> Vec<Line<'static>> {
    if width < TWO_COLUMNS {
        return group_lines(&["write", "move", "turn", "sessions"], width);
    }
    let cell_width = width.saturating_sub(GUTTER) / 2;
    let left = group_lines(&["write", "move"], cell_width);
    let right = group_lines(&["turn", "sessions"], cell_width);
    (0..left.len().max(right.len()))
        .map(|row| {
            let first = left.get(row).map(ToString::to_string).unwrap_or_default();
            let second = right.get(row).map(ToString::to_string).unwrap_or_default();
            Line::from(Span::styled(
                format!("{}{}{second}", pad(&first, cell_width), " ".repeat(GUTTER))
                    .trim_end()
                    .to_string(),
                theme::dim(),
            ))
        })
        .collect()
}

fn group_lines(groups: &[&str], width: usize) -> Vec<Line<'static>> {
    groups
        .iter()
        .flat_map(|group| {
            let heading = Line::from(Span::styled(group.to_string(), theme::dim()));
            std::iter::once(heading)
                .chain(
                    BINDINGS
                        .iter()
                        .filter(|binding| binding.group == *group)
                        .flat_map(|binding| {
                            crate::wrap::wrap(
                                &Line::from(Span::styled(
                                    cell(binding, key_column()),
                                    theme::dim(),
                                )),
                                width,
                            )
                        }),
                )
                .collect::<Vec<_>>()
        })
        .collect()
}

fn pad(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
}

fn key_column() -> usize {
    BINDINGS.iter().map(|b| b.keys.width()).max().unwrap_or(0)
}

fn cell(binding: &Binding, key_column: usize) -> String {
    format!(
        "{:<key_column$}  {}",
        binding.keys,
        binding.description,
        key_column = key_column
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_small_terminal_shows_every_binding_and_the_footer() {
        use crate::test_support::*;
        let state = state();
        let (mut ui, now) = scene();
        shown(&mut ui, crate::ui::Open::Help, now);
        let small = draw_sized(80, 24, &state, &mut ui, now);
        for binding in crate::keys::BINDINGS {
            assert!(
                small.contains(binding.keys),
                "missing {}: {small}",
                binding.keys
            );
        }
        assert!(
            small.contains("/ for commands · ! for a shell line"),
            "{small}"
        );
    }

    #[test]
    fn rough_edges_help_groups_keys_without_clipping_at_100() {
        let lines = help_lines(96);
        let text = lines
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        for group in ["write", "move", "turn", "sessions"] {
            assert!(text.contains(group), "{text}");
        }
        assert!(!text.contains('…'), "{text}");
    }

    use super::*;

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

    #[test]
    fn a_narrow_terminal_stacks_the_groups() {
        let lines = help_lines(79);
        assert_eq!(lines.first().unwrap().to_string(), "write");
        assert!(lines.iter().any(|line| line.to_string() == "sessions"));
    }

    #[test]
    fn a_wide_terminal_pairs_the_groups_without_losing_words() {
        for width in [100, 120, 160] {
            let lines = help_lines(width);
            assert!(lines[0].to_string().contains("turn"));
            assert!(lines.iter().all(|line| line.width() <= width));
            let text = lines
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            for binding in BINDINGS {
                assert!(text.contains(binding.keys), "missing {}", binding.keys);
            }
            assert!(!text.contains('…'));
        }
    }

    /// The order `esc` is obeyed in is the order the help prints, because
    /// there is only one of it.
    #[test]
    fn the_help_prints_the_escape_stack() {
        let stack = ESCAPES
            .iter()
            .map(|(_, word)| *word)
            .collect::<Vec<_>>()
            .join(" → ");
        let esc = BINDINGS
            .iter()
            .find(|binding| binding.keys == "esc")
            .expect("a row for esc");
        assert_eq!(esc.description, stack);
        assert!(
            help_lines(120)
                .iter()
                .any(|line| line.to_string().contains(&stack)),
            "the sheet prints it"
        );
    }

    /// One test per rung: what is innermost is what closes.
    #[test]
    fn esc_closes_the_innermost_thing_that_is_open() {
        let all = Open {
            sheet: true,
            card: true,
            dropdown: true,
            busy: true,
        };
        assert_eq!(escape(all), Some(Escape::Sheet));
        assert_eq!(
            escape(Open {
                sheet: false,
                ..all
            }),
            Some(Escape::Card)
        );
        assert_eq!(
            escape(Open {
                sheet: false,
                card: false,
                ..all
            }),
            Some(Escape::Dropdown)
        );
        assert_eq!(
            escape(Open {
                busy: true,
                ..Open::default()
            }),
            Some(Escape::Interrupt)
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
                busy,
                typing,
                armed,
            })
        };
        assert_eq!(pressed(true, true, true), Interrupt::Turn);
        assert_eq!(pressed(false, true, true), Interrupt::Clear);
        assert_eq!(pressed(false, false, true), Interrupt::Exit);
        assert_eq!(pressed(false, false, false), Interrupt::Arm);
    }

    #[test]
    fn no_row_overflows_the_width_it_was_built_for() {
        for width in [40usize, 80, 120, 200] {
            for line in help_lines(width) {
                assert!(line.to_string().width() <= width, "width {width}");
            }
        }
    }
}

#[cfg(test)]
mod m148 {
    use crate::test_support::*;
    #[test]
    fn m148_control_p_is_unbound_and_absent_from_help() {
        let (mut ui, now) = scene();
        let state = state();
        assert!(crate::input::on_key(&mut ui, &solo(&state), ctrl('p'), now).is_empty());
        assert!(!ui.focus.showing());
        let help = super::help_lines(200)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!help.contains(concat!("ctrl", "+p")));
    }
}
