//! The single surface owner shared by keys and pointer events.

use crate::search::Search;
use crate::select::Run;
use crate::ui::Layer;
use bingo_sdk::ItemId;

#[derive(Debug, Default)]
pub enum Focus {
    #[default]
    Composer,
    Select(Run),
    Search(Search),
    Layer(Layer),
    Row(ItemId),
    Pane(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Composer,
    Select,
    Search,
    Layer,
    Row,
    Interaction,
    Pane,
    Menu,
}

pub fn owner(ui: &crate::ui::Ui, tree: &crate::tree::Tree) -> Owner {
    resolve(
        &ui.focus,
        tree.open_interaction().is_some() && !ui.dialog.aside,
        !ui.suggestions(&tree.viewed().summary.cwd, &crate::mentions::targets(tree))
            .is_empty(),
    )
}

fn resolve(focus: &Focus, interaction: bool, menu: bool) -> Owner {
    match focus {
        Focus::Select(_) => Owner::Select,
        Focus::Search(_) => Owner::Search,
        Focus::Layer(layer) if layer.showing() => Owner::Layer,
        Focus::Row(_) => Owner::Row,
        _ if interaction => Owner::Interaction,
        Focus::Pane(_) => Owner::Pane,
        _ if menu => Owner::Menu,
        _ => Owner::Composer,
    }
}

impl Focus {
    pub fn search(&self) -> Option<&Search> {
        if let Self::Search(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn search_mut(&mut self) -> Option<&mut Search> {
        if let Self::Search(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn selection(&self) -> Option<Run> {
        if let Self::Select(value) = self {
            Some(*value)
        } else {
            None
        }
    }
    pub fn selection_mut(&mut self) -> Option<&mut Run> {
        if let Self::Select(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn row(&self) -> Option<&ItemId> {
        if let Self::Row(value) = self {
            Some(value)
        } else {
            None
        }
    }
    pub fn open(&self) -> &crate::ui::Open {
        if let Self::Layer(value) = self {
            &value.open
        } else {
            &crate::ui::Open::Nothing
        }
    }
    pub fn open_mut(&mut self) -> Option<&mut crate::ui::Open> {
        if let Self::Layer(value) = self {
            Some(&mut value.open)
        } else {
            None
        }
    }
    pub fn show(&mut self, open: crate::ui::Open, now: std::time::Instant) {
        let mut layer = Layer::shut(now);
        layer.show(open, now);
        *self = Self::Layer(layer);
    }
    pub fn close(&mut self, now: std::time::Instant) {
        if let Self::Layer(layer) = self {
            layer.close(now);
        }
    }

    pub fn showing(&self) -> bool {
        matches!(self, Self::Layer(layer) if layer.showing())
    }
    pub fn captures(&self) -> bool {
        self.showing() && self.open().captures()
    }
    pub fn drawn(&self, now: crate::clock::Now) -> Option<crate::layers::Reveal> {
        if let Self::Layer(layer) = self {
            layer.drawn(now)
        } else {
            None
        }
    }
    #[cfg(test)]
    pub fn reveal(&self, now: crate::clock::Now) -> crate::layers::Reveal {
        self.drawn(now)
            .unwrap_or_else(|| crate::layers::Reveal::none(0))
    }
    pub fn next_change(&self, now: crate::clock::Now) -> Option<std::time::Instant> {
        if let Self::Layer(layer) = self {
            layer.next_change(now)
        } else {
            None
        }
    }
    pub fn expire(&mut self, now: crate::clock::Now) {
        let over = match self {
            Self::Layer(layer) => layer.drawn(now).is_none(),
            _ => false,
        };
        if over {
            *self = Self::Composer;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::scene;
    use crate::ui::Open;

    #[test]
    fn focus_has_one_owner_for_every_key_class_and_left_press() {
        let (_, now) = scene();
        let mut layer = Layer::shut(now.instant);
        layer.show(Open::Rewind(Default::default()), now.instant);
        let cases = [
            (Focus::Composer, Owner::Composer),
            (
                Focus::Select(Run {
                    anchor: Default::default(),
                    head: Default::default(),
                }),
                Owner::Select,
            ),
            (Focus::Search(Search::open()), Owner::Search),
            (Focus::Layer(layer), Owner::Layer),
            (Focus::Row(ItemId::from_raw("itm_focus")), Owner::Row),
        ];
        for (focus, expected) in cases {
            for event in [
                "printable",
                "esc",
                "tab",
                "chord",
                "arrow",
                "enter",
                "left press",
            ] {
                assert_eq!(resolve(&focus, false, false), expected, "{event}");
                if expected != Owner::Composer {
                    assert_eq!(resolve(&focus, true, true), expected, "{event}");
                }
            }
        }
        assert_eq!(resolve(&Focus::Composer, true, true), Owner::Interaction);
        assert_eq!(resolve(&Focus::Composer, false, true), Owner::Menu);
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::test_support::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    #[test]
    fn focus_dispatch_keeps_stale_transcript_targets_behind_the_owner() {
        let mut state = folded(vec![frame(
            1,
            bingo_sdk::Event::ItemCompleted {
                item: user("itm_focus", "A transcript target"),
            },
        )]);
        state.apply(&frame(
            100,
            bingo_sdk::Event::InteractionOpened {
                interaction: permission(None, None),
            },
        ));
        let tree = solo(&state);
        let (mut ui, now) = scene();
        render_tree(&tree, &mut ui, now);
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: ui.shown.regions.transcript.x,
            row: ui.shown.regions.transcript.y + ui.shown.row_of(0).expect("a transcript row"),
            modifiers: KeyModifiers::NONE,
        };
        ui.focus = Focus::Search(Search::open());
        assert!(crate::pointer::on_mouse(&mut ui, &tree, mouse, now).is_empty());
        assert!(ui.focus.selection().is_none());
        ui.focus
            .show(crate::ui::Open::Rewind(Default::default()), now.instant);
        assert_eq!(owner(&ui, &tree), Owner::Layer);
        assert!(crate::pointer::on_mouse(&mut ui, &tree, mouse, now).is_empty());
        assert_eq!(owner(&ui, &tree), Owner::Layer);
        ui.focus.close(now.instant);
        assert_eq!(owner(&ui, &tree), Owner::Interaction);
    }
}
