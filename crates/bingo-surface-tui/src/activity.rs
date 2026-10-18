//! Waiting, plugin activity placements and queued input beneath the stream.
use ratatui::text::{Line, Span};

use crate::clock::Now;
use crate::tree::{self, Scope, Tree, Wants};
use crate::ui::Ui;
use crate::{theme, views};

/// Background waits, placed activity, queued input and embers in the dock.
/// The stream owns the live turn's row.
pub(crate) fn components(tree: &Tree, ui: &Ui, width: usize, now: Now) -> views::Drawing {
    crate::layout::count::band(2);
    let state = tree.viewed();
    let mut out = waiting(tree, width, now);
    let embers = ui.embers.drawing(state, now, width, !ui.scroll.following());
    out.next = out.next.into_iter().chain(embers.next).min();
    if !out.lines.is_empty() || !embers.lines.is_empty() {
        out.lines
            .insert(0, embers.lines.into_iter().next().unwrap_or_default());
    }
    out
}

/// A wait and the plugin placements that accompany it. Queued input is separate.
pub(crate) fn dock(tree: &Tree, ui: &Ui, width: usize, now: Now) -> views::Drawing {
    let mut out = views::Drawing::default();
    let slots = bingo_sdk::slot::placed(tree.viewed());
    let running = tree.viewed().busy();
    if !ui.activity_hidden {
        for placed in &slots {
            if let bingo_sdk::slot::Slot::Activity { view, .. } = &placed.slot {
                let Some(view) = ui.placed.pulled.view(view, tree.viewed(), width as u16) else {
                    out.lines.push(Line::from(theme::ellipsis()));
                    continue;
                };
                let view = match (running, view) {
                    (true, bingo_sdk::View::Panel { child, .. }) => child,
                    _ => view,
                };
                out.append(
                    activity_view(view, width.saturating_sub(5), now).map_lines(|lines| {
                        lines
                            .into_iter()
                            .map(|line| views::indent(line, 3))
                            .collect()
                    }),
                );
            }
        }
    }
    out
}

fn activity_view(view: &bingo_sdk::View, width: usize, now: Now) -> views::Drawing {
    use bingo_sdk::View;
    match view {
        View::Panel { title, child } => {
            let mut out = vec![Line::from(Span::styled(title.clone(), theme::dim()))];
            activity_view(child, width, now).map_lines(|lines| {
                out.extend(lines);
                out
            })
        }
        View::Box { children, .. } => children
            .iter()
            .map(|v| activity_view(v, width, now))
            .collect(),
        View::Tree { nodes } if nodes.iter().all(|n| n.children.is_empty()) => nodes
            .iter()
            .map(|node| activity_node(node, width))
            .collect::<Vec<_>>()
            .into(),
        View::Text { children, style } => {
            let mut style = style.clone();
            if style.color.is_none() {
                style.color = Some("dim".into());
            }
            views::marked(
                &View::Text {
                    children: children.clone(),
                    style,
                },
                width,
                &views::Marks::at(now),
            )
        }
        _ => views::marked(view, width, &views::Marks::at(now)),
    }
}

fn activity_node(node: &bingo_sdk::TreeNode, width: usize) -> Line<'static> {
    use bingo_sdk::Tone;
    use unicode_width::UnicodeWidthStr;
    let (index, mark, words) = match node.tone {
        Tone::Neutral => (0, theme::text(), theme::text()),
        Tone::Attention => (1, theme::presence(), theme::text().patch(theme::bold())),
        Tone::Good => (2, theme::good(), theme::dim().patch(theme::struck())),
        Tone::Bad => (0, theme::bad(), theme::bad()),
    };
    let glyph = theme::tasks()[index];
    let mut spans = vec![
        Span::styled(format!("{glyph} "), mark),
        Span::styled(
            views::clip(&node.label, width.saturating_sub(glyph.width() + 1)),
            words,
        ),
    ];
    if let Some(badge) = &node.badge {
        spans.push(Span::styled(format!(" — {badge}"), theme::dim()));
    }
    Line::from(spans)
}

/// Background agents can still be working after the viewed turn has ended.
fn waiting(tree: &Tree, width: usize, now: Now) -> views::Drawing {
    let viewed = tree.viewed();
    if viewed.busy() {
        return Default::default();
    }
    let Some(running) = tree::count(tree, Wants::Running, Scope::Under(&viewed.summary.id)) else {
        return Default::default();
    };
    let view = bingo_sdk::View::Text {
        children: vec![bingo_sdk::view::TextChild::String(format!(
            " {} ",
            theme::mark()
        ))],
        style: bingo_sdk::view::TextStyle {
            color: Some("presence".into()),
            motion: Some(Box::new(bingo_sdk::view::TextMotion::Breathe {
                with: "dim".into(),
                period_ms: 2200,
                depth: 1.0,
            })),
            ..Default::default()
        },
    };
    views::marked(&view, width, &views::Marks::at(now)).map_lines(|mut lines| {
        if let Some(line) = lines.first_mut() {
            line.spans
                .push(Span::styled(waiting_for(running), theme::text()));
        }
        lines
    })
}

/// The row's words, in the number of agents there are.
fn waiting_for(running: usize) -> String {
    format!(
        "Waiting for {running} background agent{} to finish",
        if running == 1 { "" } else { "s" }
    )
}

#[cfg(test)]
pub(crate) fn lines(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Vec<Line<'static>> {
    components(tree, ui, width, now).lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokens_do_not_compete_with_the_activity_clock() {
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                bingo_sdk::Event::TurnUsage {
                    turn: bingo_sdk::TurnId::from_raw("trn_1"),
                    usage: bingo_sdk::Usage {
                        output_tokens: 12,
                        ..Default::default()
                    },
                    context: Default::default(),
                },
            ),
        ]);
        let (mut ui, now) = scene();
        let screen = draw_tree(80, 24, &solo(&state), &mut ui, later(now, 4_000));
        assert!(!screen.contains("↓ 12"), "{screen}");
        assert!(screen.contains("working"), "{screen}");
        assert!(!screen.contains("0.0k"), "{screen}");
    }

    use super::*;
    use crate::test_support::*;

    /// The band's one row, as the terminal drew it, without the quotes a test
    /// backend prints around each row or the padding it is drawn into.
    fn row(tree: &Tree, now: Now) -> String {
        let (mut ui, _) = scene();
        draw_tree(80, 24, tree, &mut ui, now)
            .lines()
            .find(|line| line.contains("Waiting for"))
            .map(|line| line.trim_matches('"').trim_end().to_string())
            .unwrap_or_default()
    }

    /// What an idle session says while the agents it started are still at
    /// work, counted the way a person counts (M82).
    #[test]
    fn a_session_idle_over_running_agents_says_how_many_it_waits_for() {
        let (_, now) = scene();
        let one = row(&waited_on(1), now);
        assert!(
            one.ends_with("Waiting for 1 background agent to finish"),
            "{one}"
        );
        let two = row(&waited_on(2), now);
        assert!(
            two.ends_with("Waiting for 2 background agents to finish"),
            "{two}"
        );
    }

    /// The wait is not a turn, so it wears none of a turn's furniture — every
    /// piece of which rides in the row's one pair of parentheses.
    #[test]
    fn the_wait_carries_no_key_no_clock_and_no_token_count() {
        let (_, now) = scene();
        let drawn = row(&waited_on(1), now);
        assert!(!drawn.contains('('), "{drawn}");
    }

    /// Nothing at work, nothing to wait for: an agent that has finished is
    /// not waited on, and a session with none never was.
    #[test]
    fn a_session_with_no_agent_at_work_says_nothing_at_all() {
        let (_, now) = scene();
        assert_eq!(row(&waited_on(0), now), "");
        let mut done = waited_on(1);
        done.fold.apply(&agent_frame(
            2,
            3,
            completed("trn_9", bingo_sdk::TurnStatus::Completed),
        ));
        assert_eq!(row(&done, now), "");
    }

    /// The row counts what hangs under this session, not everything else
    /// that is running: a child looking up at a running parent is waiting on
    /// nothing of its own. The crew strip likewise names only descendants.
    #[test]
    fn a_child_under_a_running_root_is_not_waiting_on_an_agent() {
        let (mut ui, now) = scene();
        let mut tree = folded_tree(vec![
            frame(1, started("trn_1")),
            child_frame(1, announced("reviewer")),
        ]);
        tree.show(&child_id());
        assert_eq!(row(&tree, now), "");
        let drawn = draw_tree(80, 24, &tree, &mut ui, now);
        assert!(!drawn.contains("running"), "{drawn}");
    }

    /// The wake that ends the wait opens a turn, and the turn's own row is
    /// what the band then says: one row's slot either way (§3).
    #[test]
    fn a_turn_on_the_viewed_session_takes_the_row_back_from_the_wait() {
        let (mut ui, now) = scene();
        let mut woken = waited_on(1);
        woken.fold.apply(&frame(2, started("trn_1")));
        let at = later(now, 1_600);
        assert_eq!(row(&woken, at), "");
        let drawn = draw_tree(80, 24, &woken, &mut ui, at);
        assert!(!drawn.contains("Waiting for"), "{drawn}");
        assert!(drawn.contains("working"), "{drawn}");
    }
}

#[cfg(test)]
mod ember_tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn embers_keep_their_row_and_deadline_without_a_dock_spinner() {
        let (mut ui, now) = scene();
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                bingo_sdk::Event::ItemStarted {
                    item: assistant("answer", &"a".repeat(1024), bingo_sdk::ItemStatus::Running),
                },
            ),
        ]);
        ui.embers.observe(0, &state, now.instant);
        let tree = solo(&state);
        let row = components(&tree, &ui, 80, now);
        assert!(
            row.lines
                .iter()
                .any(|line| line.to_string().contains("· · · ·"))
        );
        assert_eq!(row.next, Some(now.instant + crate::clock::FRAME));
        let gone = components(&tree, &ui, 80, later(now, 600));
        assert!(gone.lines.is_empty());
        assert_eq!(gone.next, None);
        assert!(components(&tree, &ui, 80, still(now)).lines.is_empty());
    }
}
