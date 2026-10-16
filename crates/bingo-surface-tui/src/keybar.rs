//! One fixed row, projected from the current owner and the live keymap.
mod sealed;
mod site;
use crate::{
    clock::Now,
    layout::Layout,
    theme,
    tree::{Scope, Tree, Wants},
    ui::Ui,
};
use bingo_sdk::component::{KeyBarProps, KeyPair, Props};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

pub fn draw(tree: &Tree, ui: &Ui, frame: &mut Layout, now: Now) -> Line<'static> {
    let width = usize::from(frame.area().width);
    let mut pairs = if let Some((_, ask)) = tree.open_interaction() {
        sealed::pairs(ui, ask, question_focus(ui, frame, ask))
    } else {
        site::hints(tree.viewed(), ui, frame)
            .map_or_else(|| resting(tree, ui, now), |hints| hints.pairs)
    };
    attention(tree, ui, now, &mut pairs);
    let props = Props::KeyBar(KeyBarProps {
        pairs,
        toast: crate::toast::components(ui, tree.viewed(), now, width, &mut frame.pulls),
    });
    if tree.open_interaction().is_some() {
        return own(&props, width).remove(0);
    }
    let key = crate::component::key(tree.viewed(), "KeyBar", &props, width as u16, 1);
    let answer = key.as_ref().and_then(|key| ui.placed.pulled.get(key));
    if let Some(key) = key {
        frame.pulls.push(key);
    }
    crate::component::draw(&props, answer, width, own)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn resting(tree: &Tree, ui: &Ui, now: Now) -> Vec<KeyPair> {
    if ui.exit_armed(now.instant) {
        return vec![KeyPair::new("ctrl+c", "again to exit")];
    }
    if tree.viewed().busy() {
        vec![
            KeyPair::new("esc", "stop"),
            KeyPair::new("⏎", "queue a message"),
        ]
    } else {
        let mut pairs = vec![KeyPair::new("/", "commands"), KeyPair::new("?", "keys")];
        let bindings = crate::keys::commands::bindings(ui, tree.viewed());
        let sessions = bindings
            .commands
            .iter()
            .find(|(_, line)| line.as_str() == "/sessions" || line.as_str() == "/resume");
        pairs.push(KeyPair::new(
            sessions.map_or("ctrl+r", |(key, _)| key),
            "sessions",
        ));
        pairs.push(KeyPair::new("shift+tab", "mode"));
        if let Some((key, _)) = bindings
            .commands
            .iter()
            .find(|(_, line)| line.as_str() == "/model next")
        {
            pairs.push(KeyPair::new(key, "next model"));
        }
        pairs
    }
}

fn attention(tree: &Tree, ui: &Ui, now: Now, pairs: &mut Vec<KeyPair>) {
    for placed in bingo_sdk::slot::placed(tree.viewed()) {
        if let bingo_sdk::slot::Slot::Status { text, until, .. } = placed.slot {
            let text = match until {
                Some(at) if at > now.wall => format!(
                    "{text} in {}",
                    crate::clock::span(at.duration_since(now.wall).unsigned_abs())
                ),
                Some(_) => continue,
                None => text,
            };
            pairs.push(KeyPair::new("", text));
        }
    }
    let waiting = ui
        .catalogs
        .plugins
        .iter()
        .filter(|entry| entry.meta["state"]["kind"] == "waiting")
        .count();
    if waiting > 0 {
        pairs.push(KeyPair::new(
            "/plugins",
            format!("{waiting} from this repo waiting"),
        ));
    }
    if let Some(count) = crate::tree::count(tree, Wants::Attention, Scope::Others) {
        pairs.push(KeyPair {
            emphasis: true,
            ..KeyPair::new("ctrl+g", format!("{count} need you"))
        });
    }
}

pub(super) fn own(props: &Props, width: usize) -> Vec<Line<'static>> {
    let Props::KeyBar(props) = props else {
        return vec![Line::default()];
    };
    if let Some(toast) = &props.toast {
        let mut spans = vec![Span::styled(
            format!(" {}", toast.text.lines().collect::<Vec<_>>().join(" · ")),
            theme::level(toast.level),
        )];
        if let Some(about) = &toast.about {
            spans.push(Span::styled(format!(" · {about}"), theme::dim()));
        }
        return vec![Line::from(crate::status::clip(
            spans,
            width.saturating_sub(1),
        ))];
    }
    let mut used = 0;
    let mut start = props.pairs.len();
    for (index, pair) in props.pairs.iter().enumerate().rev() {
        let size = pair.keys.width()
            + pair.action.width()
            + usize::from(!pair.keys.is_empty())
            + usize::from(used > 0) * 3;
        if used + size > width.saturating_sub(3) {
            break;
        }
        used += size;
        start = index;
    }
    let mut spans = vec![Span::raw(" ".repeat(width.saturating_sub(used + 2)))];
    for (index, pair) in props.pairs[start..].iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" · ", theme::ash()));
        }
        spans.push(Span::styled(
            pair.keys.clone(),
            theme::glow().patch(theme::bold()),
        ));
        spans.push(Span::styled(
            format!(
                "{}{}",
                if pair.keys.is_empty() { "" } else { " " },
                pair.action
            ),
            if pair.emphasis {
                theme::dim().patch(theme::bold())
            } else {
                theme::dim()
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
    if ui.dialog.current().is_some() && !ui.dialog.aside {
        return bingo_sdk::component::ComposerProps {
            tag: String::new(),
            placeholder: "answer above · typing here leaves it for later".into(),
        };
    }
    site::hints(state, ui, frame)
        .and_then(|hints| hints.composer)
        .unwrap_or_else(|| bingo_sdk::component::ComposerProps {
            tag: String::new(),
            placeholder: if ui.dialog.aside {
                "your message waits until the ask is answered"
            } else if state.summary.driver == bingo_sdk::Driver::Log {
                "post to the room"
            } else {
                crate::keys::PLACEHOLDER
            }
            .into(),
        })
}

#[cfg(test)]
pub fn hints(
    tree: &Tree,
    ui: &Ui,
    width: usize,
    now: Now,
    _: &mut Vec<crate::pulled::Key>,
) -> Line<'static> {
    let mut pairs = resting(tree, ui, now);
    attention(tree, ui, now, &mut pairs);
    own(
        &Props::KeyBar(KeyBarProps {
            pairs,
            toast: crate::toast::current(ui, tree.viewed(), now),
        }),
        width,
    )
    .remove(0)
}

#[cfg(test)]
mod tests;

fn question_focus<'a>(
    ui: &'a Ui,
    frame: &Layout,
    ask: &bingo_sdk::Interaction,
) -> Option<&'a bingo_sdk::component::QuestionFocus> {
    let key = frame
        .pulls
        .iter()
        .find(|key| key.request.request_id == ask.id.as_str())?;
    let bingo_sdk::View::Box { layout, .. } = ui.placed.pulled.get(key)? else {
        return None;
    };
    layout.question.as_ref()
}

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
