//! The unified diff a change to an entry shows twice: once on the
//! permission card, once in the result every surface renders. One
//! rendering, so what a person approved is what they later read.

use std::path::Path;

use similar::{Algorithm, TextDiff};

pub fn unified(path: &Path, old: &str, new: &str) -> String {
    let shown = path.display().to_string();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Histogram)
        .diff_lines(old, new);
    diff.unified_diff().header(&shown, &shown).to_string()
}
