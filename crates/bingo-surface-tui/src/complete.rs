//! `@` in the composer: a name this session can reach, or a path from its own
//! directory.
//!
//! The walk obeys `.gitignore` — a repository offers its sources and not its
//! build. The list that ranks and draws the paths is [`crate::mention_list`].
//!
//! A completed mention keeps its `@`, so the line itself says which of its
//! words are names and which are paths, and [`attachments`] is derived from it
//! rather than remembered beside it — a name is not a picture.

use std::path::Path;

use ignore::WalkBuilder;

/// How many paths are walked before the walk gives up. A directory larger than
/// this is one nobody completes in anyway, and a bounded walk is what keeps the
/// first `@` inside a frame.
const CAP: usize = 20_000;

/// Every file under `cwd` a person could mean, in the order the walk finds
/// them, relative to `cwd`.
pub fn walk(cwd: &Path) -> Vec<String> {
    WalkBuilder::new(cwd)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        // A directory with an ignore file in it means it whether or not git
        // has ever been run there.
        .require_git(false)
        .parents(false)
        // The filesystem's own order is nobody's: two runs in the same
        // directory offer the same eight rows in the same places.
        .sort_by_file_path(Ord::cmp)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.path().strip_prefix(cwd).ok().map(mentioned))
        .take(CAP)
        .collect()
}

/// A path as a mention spells it: a word in a line, written the same on every
/// platform, so its parts are joined by `/` whatever the system joins them by.
fn mentioned(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The mention under the caret, without its `@`, when the word being typed is
/// one. The word is what follows the last space, so a completed mention with a
/// space after it is finished and offers nothing.
pub fn mention(line: &str) -> Option<&str> {
    word(line).strip_prefix('@')
}

/// The line with its half-typed mention replaced by this path, and a space
/// after it so the next word starts clean.
pub fn completed(line: &str, path: &str) -> String {
    let head = match line.rfind(char::is_whitespace) {
        Some(at) => &line[..=at],
        None => "",
    };
    format!("{head}@{path} ")
}

/// The pictures a line mentions, which is what reaches the model beside it,
/// as they were typed. The line is the only record: nothing is remembered
/// when a mention is deleted. What counts as one is `bingo-pictures`'
/// (ADR-0041) — every extension a decoder knows, and any URL, whose bytes are
/// read at submit and whose type is decided there.
pub fn attachments(line: &str) -> Vec<String> {
    line.split_whitespace()
        .filter_map(|word| word.strip_prefix('@'))
        .filter(|word| bingo_pictures::names_a_picture(word))
        .map(str::to_owned)
        .collect()
}

/// The word being typed: everything after the last space.
fn word(line: &str) -> &str {
    match line.rfind(char::is_whitespace) {
        Some(at) => &line[at + 1..],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\n").expect("a manifest");
        std::fs::write(dir.path().join(".gitignore"), "target\n").expect("an ignore file");
        std::fs::create_dir_all(dir.path().join("src")).expect("a source directory");
        std::fs::write(dir.path().join("src/lib.rs"), "//! it\n").expect("a source");
        std::fs::create_dir_all(dir.path().join("target/debug")).expect("a build directory");
        std::fs::write(dir.path().join("target/debug/bingo"), "elf").expect("a binary");
        std::fs::write(dir.path().join("shot.png"), "png").expect("a picture");
        dir
    }

    fn walked(dir: &tempfile::TempDir) -> Vec<String> {
        let mut paths = walk(dir.path());
        paths.sort();
        paths
    }

    #[test]
    fn the_walk_offers_the_sources_and_not_the_build() {
        let dir = tree();
        assert_eq!(
            walked(&dir),
            vec![
                "Cargo.toml".to_string(),
                "shot.png".to_string(),
                "src/lib.rs".to_string(),
            ],
            "`.gitignore` is obeyed and the hidden files are not offered"
        );
    }

    #[test]
    fn a_mention_is_the_word_being_typed_and_a_finished_one_is_not() {
        assert_eq!(mention("@Car"), Some("Car"));
        assert_eq!(mention("look at @src/li"), Some("src/li"));
        assert_eq!(mention("@"), Some(""), "the bare mark offers everything");
        assert_eq!(mention("@src/lib.rs "), None, "a space finishes it");
        assert_eq!(mention("nothing to complete"), None);
        assert_eq!(mention("mail@example.com"), None, "not the word's opening");
    }

    #[test]
    fn a_completion_replaces_the_mention_and_leaves_the_rest_alone() {
        assert_eq!(completed("@Car", "Cargo.toml"), "@Cargo.toml ");
        assert_eq!(completed("read @src/li", "src/lib.rs"), "read @src/lib.rs ",);
    }

    #[test]
    fn only_a_picture_becomes_an_attachment() {
        assert_eq!(
            attachments("look at @shot.png beside @src/lib.rs"),
            vec!["shot.png".to_string()],
        );
        assert_eq!(attachments("@SHOT.JPEG"), vec!["SHOT.JPEG".to_string()]);
        assert!(attachments("no mention here").is_empty());
    }

    /// Every format the decoder reads, not the four a provider takes: what a
    /// person has on disk is wider than what goes on the wire (ADR-0041).
    #[test]
    fn a_format_only_the_decoder_reads_is_an_attachment_too() {
        assert_eq!(
            attachments("@shot.bmp @scan.tiff @icon.ico"),
            vec![
                "shot.bmp".to_string(),
                "scan.tiff".to_string(),
                "icon.ico".to_string()
            ],
        );
    }

    /// A URL is mentioned, never completed: nothing on this machine could
    /// have offered it, and its type is the bytes' to say, not its name's.
    #[test]
    fn a_url_is_an_attachment_whatever_it_ends_in() {
        assert_eq!(
            attachments("look at @https://x/y.jpg and @http://x/picture"),
            vec![
                "https://x/y.jpg".to_string(),
                "http://x/picture".to_string()
            ],
        );
    }
}
