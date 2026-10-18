//! The one line a filesystem call's step shows on its right (ADR-0095 §9). A
//! change to an existing file needs none: its diff is the fact, and the
//! counts are read off it.

use bingo_sdk::Upshot;

pub(crate) fn read(text: &str) -> Upshot {
    let count = text.lines().count();
    let noun = if count == 1 { "line" } else { "lines" };
    Upshot::plain(format!("{count} {noun}"))
}

pub(crate) fn picture(size: Option<(u32, u32)>) -> Option<Upshot> {
    let (width, height) = size?;
    Some(Upshot::plain(format!("{width}×{height}")))
}

/// How many entries a search found, in the caller's plural noun.
pub(crate) fn found(count: usize, plural: &str) -> Upshot {
    match count {
        0 => Upshot::plain("none"),
        1 => Upshot::plain(format!("1 {}", plural.trim_end_matches('s'))),
        many => Upshot::plain(format!("{many} {plural}")),
    }
}

/// A file written where there was none: every line of it is new.
pub(crate) fn created(content: &str) -> Upshot {
    Upshot::Diff {
        added: u32::try_from(content.lines().count()).unwrap_or(u32::MAX),
        removed: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_counts_in_its_own_noun() {
        assert_eq!(found(0, "files"), Upshot::plain("none"));
        assert_eq!(found(1, "files"), Upshot::plain("1 file"));
        assert_eq!(found(12, "lines"), Upshot::plain("12 lines"));
    }

    #[test]
    fn a_new_file_adds_every_line() {
        assert_eq!(
            created("a\nb\nc\n"),
            Upshot::Diff {
                added: 3,
                removed: 0
            }
        );
    }
}
