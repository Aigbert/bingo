//! Input state and gestures, owned by the surface's one focus.

use super::{Catalogs, Menu};
use crate::composer::Composer;
use crate::dialog::Dialog;
use crate::focus::Focus;
use crate::history::PromptHistory;
use crate::pictures::Held;
use crate::select::{self, Cell, Select};
use bingo_sdk::TurnId;
use std::time::Instant;

#[derive(Debug)]
pub struct Controls {
    /// `ctrl+t` took the task list off the band (M74). The verb row keeps
    /// the task being done either way: hiding the list is about rows, not
    /// about what the turn is doing.
    pub activity_hidden: bool,

    pub actions: crate::actions::Queue,
    pub composer: Composer,
    /// The pasted pictures behind the composer's line, by their `[image N]`.
    pub pictures: Held,
    pub history: PromptHistory,
    pub queue_arrivals: crate::queue_rows::Arrivals,
    pub dialog: Dialog,
    pub last_typed: Option<jiff::Timestamp>,
    pub menu: Menu,
    /// What the transcript is holding: a focused block, a run of cells.
    pub select: Select,
    /// The one owner of keys and pointer events.
    pub focus: Focus,
    /// When ctrl+c was pressed on an empty composer.
    pub armed: Option<Instant>,
    /// The turn this surface has asked to stop, so the activity row answers
    /// the key on the very frame it was pressed (§7). The kernel decides what
    /// an interrupt does and its `TurnCompleted` is the end of the story; this
    /// is a fact about the keypress, not a copy of session state, and it is
    /// kept by turn id so it can never speak for the turn after this one.
    pub stop_asked: Option<TurnId>,
    /// The frame clock of an esc that closed nothing.
    pub escaped_at: Option<Instant>,
    /// What the kernel offers the dropdown, read once at start.
    pub catalogs: Catalogs,
    /// An `open` is in flight; the swap happens when it lands.
    pub opening: bool,
}

impl Controls {
    pub fn older(&mut self) -> Option<String> {
        self.history.older(self.composer.text())
    }
    pub fn hold_picture(&mut self, path: std::path::PathBuf) -> u32 {
        self.pictures.hold(self.composer.text(), path)
    }
    pub fn paste_picture(&mut self, path: std::path::PathBuf) {
        let number = self.hold_picture(path);
        self.composer.picture(number, &self.pictures);
    }
    pub fn new(history: Vec<String>) -> Self {
        Self {
            actions: Default::default(),
            activity_hidden: false,
            composer: Composer::default(),
            pictures: Held::default(),
            history: PromptHistory::new(history),
            queue_arrivals: Default::default(),
            dialog: Dialog::default(),
            last_typed: None,
            menu: Menu::default(),
            select: Select::default(),
            focus: Focus::Composer,
            armed: None,
            stop_asked: None,
            escaped_at: None,
            catalogs: Catalogs::default(),
            opening: false,
        }
    }
}

impl Controls {
    pub fn start_selection(&mut self, at: Cell) {
        self.focus = Focus::Select(select::Run {
            anchor: at,
            head: at,
        });
        self.select.dragging = None;
    }
    pub fn extend_selection(&mut self, to: Cell) {
        if let Some(run) = self.focus.selection_mut() {
            run.head = to;
        }
    }
    pub fn walk_selection(&mut self, lines: isize, columns: isize, height: usize) {
        if let Some(run) = self.focus.selection_mut() {
            run.walk(lines, columns, height);
        }
    }
    pub fn clear_selection(&mut self) {
        if matches!(self.focus, Focus::Select(_)) {
            self.focus = Focus::Composer;
        }
        self.select.dragging = None;
    }
}
