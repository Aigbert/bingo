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

fn resolve(_focus: &Focus, _interaction: bool, _menu: bool) -> Owner {
    Owner::Composer
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
