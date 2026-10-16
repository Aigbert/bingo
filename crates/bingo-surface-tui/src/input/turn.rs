//! A finished turn's folded line: what `ctrl+o` and `⏎` open on it.

use super::*;

/// A finished turn opens under its line: its steps, what they brought back,
/// its thoughts. Opened, the next press takes the line's own step whole into
/// the pager, as `ctrl+o` always goes one way.
pub(super) fn open(ui: &mut Ui, tree: &Tree, now: Now, line: bingo_sdk::ItemId) {
    let opened = ui.folds.get(&line) == Some(&fold::Fold::Open);
    match opened {
        true => {
            pager::open_block(ui, tree, now, Some(&line));
        }
        false => {
            ui.folds.insert(line, fold::Fold::Open);
        }
    }
}

/// The folded line `⏎` on the held block opens: the block is that line, and
/// its turn is not opened yet.
pub(super) fn folded_line(
    ui: &Ui,
    tree: &Tree,
    block: &bingo_sdk::ItemId,
) -> Option<bingo_sdk::ItemId> {
    let line = ui.shown.blocks.roles().line_of(tree.viewed(), block)?;
    let shut = &line == block && ui.folds.get(&line) != Some(&fold::Fold::Open);
    shut.then_some(line)
}
