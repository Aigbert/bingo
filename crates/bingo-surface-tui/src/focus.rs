//! The single surface owner shared by keys and pointer events.

use crate::opening::Playing;
use crate::rail::CardId;
use crate::search::Search;
use crate::select::Run;
use crate::ui::Layer;

#[derive(Debug, Default)]
pub enum Focus {
    #[default]
    Composer,
    Intro(Playing),
    Select(Run),
    Search(Search),
    Layer(Layer),
    Card(CardId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Composer,
    Intro,
    Select,
    Search,
    Layer,
    Card,
    Interaction,
    Menu,
}

pub fn owner(ui: &crate::ui::Ui, tree: &crate::tree::Tree) -> Owner {
    resolve(&ui.focus, tree.open_interaction().is_some(), !ui.suggestions(
        &tree.viewed().summary.cwd, &crate::mentions::targets(tree),
    ).is_empty())
}

fn resolve(focus: &Focus, interaction: bool, menu: bool) -> Owner {
    match focus {
        Focus::Intro(_) => Owner::Intro,
        Focus::Select(_) => Owner::Select,
        Focus::Search(_) => Owner::Search,
        Focus::Layer(layer) if layer.showing() && layer.open.captures() => Owner::Layer,
        Focus::Card(_) => Owner::Card,
        _ if interaction => Owner::Interaction,
        _ if menu => Owner::Menu,
        _ => Owner::Composer,
    }
}

impl Focus {
    pub fn intro(&self) -> Option<&Playing> {
        if let Self::Intro(value) = self { Some(value) } else { None }
    }
    pub fn search(&self) -> Option<&Search> {
        if let Self::Search(value) = self { Some(value) } else { None }
    }
    pub fn search_mut(&mut self) -> Option<&mut Search> {
        if let Self::Search(value) = self { Some(value) } else { None }
    }
    pub fn selection(&self) -> Option<Run> {
        if let Self::Select(value) = self { Some(*value) } else { None }
    }
    pub fn selection_mut(&mut self) -> Option<&mut Run> {
        if let Self::Select(value) = self { Some(value) } else { None }
    }
    pub fn card(&self) -> Option<&CardId> {
        if let Self::Card(value) = self { Some(value) } else { None }
    }
    pub fn open(&self) -> &crate::ui::Open {
        if let Self::Layer(value) = self { &value.open } else { &crate::ui::Open::Nothing }
    }
    pub fn open_mut(&mut self) -> Option<&mut crate::ui::Open> {
        if let Self::Layer(value) = self { Some(&mut value.open) } else { None }
    }
    pub fn show(&mut self, open: crate::ui::Open, now: std::time::Instant) {
        let mut layer = Layer::shut(now);
        layer.show(open, now);
        *self = Self::Layer(layer);
    }
    pub fn close(&mut self, now: std::time::Instant) {
        if let Self::Layer(layer) = self { layer.close(now); }
    }
    pub fn toggle(&mut self, open: crate::ui::Open, now: std::time::Instant) {
        if self.open() == &open && self.showing() { self.close(now); }
        else { self.show(open, now); }
    }
    pub fn showing(&self) -> bool {
        matches!(self, Self::Layer(layer) if layer.showing())
    }
    pub fn captures(&self) -> bool {
        self.showing() && self.open().captures()
    }
    pub fn drawn(&self, now: crate::clock::Now) -> Option<crate::layers::Reveal> {
        if let Self::Layer(layer) = self { layer.drawn(now) } else { None }
    }
    #[cfg(test)]
    pub fn reveal(&self, now: crate::clock::Now) -> crate::layers::Reveal {
        self.drawn(now).unwrap_or_else(|| crate::layers::Reveal::none(0))
    }
    pub fn next_change(&self, now: crate::clock::Now) -> Option<std::time::Instant> {
        if let Self::Layer(layer) = self { layer.next_change(now) } else { None }
    }
    pub fn expire(&mut self, now: crate::clock::Now) {
        let over = match self {
            Self::Intro(intro) => intro.over(now),
            Self::Layer(layer) => layer.drawn(now).is_none(),
            _ => false,
        };
        if over { *self = Self::Composer; }
    }
    #[cfg(test)]
    pub fn is(&self, open: &crate::ui::Open) -> bool {
        self.open() == open && self.showing()
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
        layer.show(Open::Panel, now.instant);
        let cases = [
            (Focus::Composer, Owner::Composer),
            (Focus::Intro(Playing::from(now.instant)), Owner::Intro),
            (Focus::Select(Run { anchor: Default::default(), head: Default::default() }), Owner::Select),
            (Focus::Search(Search::open()), Owner::Search),
            (Focus::Layer(layer), Owner::Layer),
            (Focus::Card(CardId { plugin: "fixture".into(), kind: "card".into(), slot: None }), Owner::Card),
        ];
        for (focus, expected) in cases {
            for event in ["printable", "esc", "tab", "chord", "arrow", "enter", "left press"] {
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
