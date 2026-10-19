//! Footer pairs projected from the current owner and the live keymap.
mod sealed;
mod site;
use crate::{clock::Now, layout::Layout, theme, tree::Tree, ui::Ui};
use bingo_sdk::component::{KeyBarProps, KeyPair, Props};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

pub fn draw(
    tree: &Tree,
    ui: &Ui,
    frame: &mut Layout,
    width: usize,
    now: Now,
) -> crate::views::Drawing {
    let pairs = if let Some((_, ask)) = tree.open_interaction() {
        sealed::pairs(
            ui,
            ask,
            crate::ask::question::focus(ui, &frame.pulls, ask),
            !tree.is_root(tree.view()),
        )
    } else {
        site::hints(tree.viewed(), ui, frame).map_or_else(|| resting(tree, ui), |hints| hints.pairs)
    };
    let pairs = pairs
        .into_iter()
        .map(|mut pair| {
            pair.keys = theme::glyphs::key_text(&pair.keys);
            pair
        })
        .collect();
    let props = Props::KeyBar(KeyBarProps { pairs });
    if tree.open_interaction().is_some() {
        return own(&props, width).into();
    }
    let key = crate::component::key(tree.viewed(), "KeyBar", &props, width as u16, 1);
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.get(key));
    if let Some(key) = key {
        frame.pulls.push(key);
    }
    crate::component::draw_marked(&props, answer, width, &crate::views::Marks::at(now), own)
        .map_lines(|lines| if width == 0 { Vec::new() } else { lines })
}

fn resting(tree: &Tree, ui: &Ui) -> Vec<KeyPair> {
    if !ui.scroll.following() {
        return vec![
            KeyPair::new("end", "latest"),
            KeyPair::new("ctrl+f", "find"),
        ];
    }
    if !tree.is_root(tree.view()) {
        return away(tree, ui);
    }
    let pairs = home(tree, ui);
    if tree.needers().next().is_none() || !ui.composer.is_empty() {
        return pairs;
    }
    let mut answer = vec![KeyPair::new("ctrl+g", "answer")];
    answer.extend(pairs.into_iter().rev().take(2).rev());
    answer
}

/// In a teammate's session or a room: esc is the way home, ctrl+c the way
/// to stop.
fn away(tree: &Tree, ui: &Ui) -> Vec<KeyPair> {
    let state = tree.viewed();
    let room = bingo_collab::room::is_room(&state.summary);
    if !ui.composer.is_empty() {
        if state.busy() {
            return vec![
                KeyPair::new("⏎", "steer"),
                KeyPair::new("tab", "queue"),
                KeyPair::new("esc", "clear"),
            ];
        }
        let send = if room { "post" } else { "send" };
        return vec![KeyPair::new("⏎", send), KeyPair::new("esc", "clear")];
    }
    let mut pairs = vec![KeyPair::new("esc", "back")];
    if tree.needers().next().is_some() {
        pairs.push(KeyPair::new("ctrl+g", "answer"));
    }
    if state.busy() {
        pairs.push(KeyPair::new("ctrl+c", "stop"));
    }
    pairs.push(if room && !bingo_collab::room::closed_of(state) {
        KeyPair::new("@", "call on")
    } else {
        KeyPair::new("/", "commands")
    });
    pairs.truncate(3);
    pairs
}

/// At home: what the turn and the draft allow.
fn home(tree: &Tree, ui: &Ui) -> Vec<KeyPair> {
    if tree.viewed().busy() {
        if !ui.composer.is_empty() {
            return vec![
                KeyPair::new("⏎", "steer"),
                KeyPair::new("tab", "queue"),
                KeyPair::new("esc", "stop"),
            ];
        }
        let mut pairs = Vec::new();
        if !tree.viewed().queue.is_empty() {
            pairs.push(KeyPair::new("↑", "edit queued"));
        }
        if crate::input::placed_action(tree.viewed(), 'b').is_some() {
            pairs.push(KeyPair::new("ctrl+b", "background"));
        }
        pairs.push(KeyPair::new("esc", "stop"));
        return pairs;
    }
    if !ui.composer.is_empty() {
        return vec![KeyPair::new("⏎", "send"), KeyPair::new("⇧⏎", "newline")];
    }
    if !tree.viewed().transcript.items().is_empty() {
        return vec![
            KeyPair::new("ctrl+o", "details"),
            KeyPair::new("/", "commands"),
            KeyPair::new("?", "keys"),
        ];
    }
    if matches!(ui.focus, crate::focus::Focus::Welcome(_)) {
        return vec![
            KeyPair::new("↑↓", "recent"),
            KeyPair::new("⏎", "resume"),
            KeyPair::new("esc", "back"),
        ];
    }
    if tree.viewed().transcript.items().is_empty() && !crate::welcome::recent(ui).is_empty() {
        return vec![
            KeyPair::new("↑", "recent"),
            KeyPair::new("/", "commands"),
            KeyPair::new("?", "keys"),
        ];
    }
    vec![
        KeyPair::new("/", "commands"),
        KeyPair::new("@", "files"),
        KeyPair::new("?", "keys"),
    ]
}

pub(super) fn own(props: &Props, width: usize) -> Vec<Line<'static>> {
    let Props::KeyBar(props) = props else {
        return vec![Line::default()];
    };
    let (pairs, _) = props.fitting(width, UnicodeWidthStr::width);
    let mut spans = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            pair.keys.clone(),
            theme::foreground("muted").unwrap_or_default(),
        ));
        spans.push(Span::styled(
            format!(
                "{}{}",
                if pair.keys.is_empty() { "" } else { " " },
                pair.action
            ),
            if pair.emphasis {
                theme::foreground("dim")
                    .unwrap_or_default()
                    .patch(theme::bold())
            } else {
                theme::foreground("dim").unwrap_or_default()
            },
        ));
    }
    vec![Line::from(spans)]
}

fn composer_props(
    state: &bingo_sdk::SessionState,
    ui: &Ui,
    frame: &Layout,
) -> bingo_sdk::component::ComposerProps {
    site::hints(state, ui, frame)
        .and_then(|hints| hints.composer)
        .unwrap_or_else(|| bingo_sdk::component::ComposerProps {
            tag: String::new(),
            placeholder: placeholder(state),
        })
}
fn placeholder(state: &bingo_sdk::SessionState) -> String {
    let name = crate::tree::name(state);
    if bingo_collab::room::is_room(&state.summary) {
        return if bingo_collab::room::closed_of(state) {
            "This room is closed".into()
        } else {
            format!("Post to {name}, @ to call on someone")
        };
    }
    if !state.interactions.is_empty() {
        let who = if state.summary.parent.is_some() {
            name.as_str()
        } else {
            "bingo"
        };
        return format!("Answer above, or type to tell {who} something else");
    }
    if state.summary.parent.is_some() {
        return if state.busy() {
            format!("Steer {name}, or queue what comes next")
        } else {
            format!("Message {name}")
        };
    }
    if state.busy() {
        "Steer, or queue what comes next"
    } else if state.transcript.items().is_empty() {
        "Ask anything, or / for commands"
    } else {
        "Reply, or / for commands"
    }
    .into()
}

#[cfg(test)]
mod tests;

pub fn composer(
    state: &bingo_sdk::SessionState,
    ui: &Ui,
    frame: &mut Layout,
) -> bingo_sdk::component::ComposerProps {
    let original = composer_props(state, ui, frame);
    let props = Props::Composer(original.clone());
    let Some(key) = crate::component::key(state, "Composer", &props, frame.area().width, 1) else {
        return original;
    };
    let changed = match ui.placed.pulled.get(&key) {
        Some(bingo_sdk::View::Engine {
            component: bingo_sdk::render::Component::Composer,
            props,
        }) => serde_json::from_value(props.clone()).ok(),
        _ => None,
    };
    frame.pulls.push(key);
    changed.unwrap_or(original)
}
