//! Embers above the dock and plugin activity placements; queue rows live in `queue_rows`.
use ratatui::text::{Line, Span};

use crate::clock::Now;
use crate::layout::grid;
use crate::tree::Tree;
use crate::ui::Ui;
use crate::{theme, views};

/// Embers above the dock. The stream owns the live turn's row, and agents at
/// work while the session is idle are the footer's lamps.
pub(crate) fn components(tree: &Tree, ui: &Ui, width: usize, now: Now) -> views::Drawing {
    crate::layout::count::band(2);
    let embers = ui
        .embers
        .drawing(tree.viewed(), now, width, !ui.scroll.following());
    views::Drawing {
        lines: embers.lines.into_iter().take(1).collect(),
        next: embers.next,
    }
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
                    activity_view(view, width.saturating_sub(grid::TX + grid::RM), now).map_lines(
                        |lines| {
                            lines
                                .into_iter()
                                .map(|line| views::indent(line, grid::TX))
                                .collect()
                        },
                    ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

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
