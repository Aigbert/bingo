//! The lines a unified diff adds and removes, read off its hunks.

/// Added and removed lines across every hunk. Each hunk is read for as many
/// lines as its header says, so a later file's `---`/`+++` header, which
/// sits outside every hunk, is not mistaken for a change.
pub(super) fn counts(unified: &str) -> (u32, u32) {
    let mut hunk = Hunk::default();
    let (mut added, mut removed) = (0, 0);
    for line in unified.lines() {
        if let Some(opened) = Hunk::open(line) {
            hunk = opened;
            continue;
        }
        match hunk.take(line) {
            Some(Mark::Added) => added += 1,
            Some(Mark::Removed) => removed += 1,
            Some(Mark::Kept) | None => {}
        }
    }
    (added, removed)
}

/// What is left of the hunk being read, on each side.
#[derive(Default)]
struct Hunk {
    old: u32,
    new: u32,
}

enum Mark {
    Added,
    Removed,
    Kept,
}

impl Hunk {
    /// `@@ -start[,length] +start[,length] @@`; a missing length is one line.
    fn open(line: &str) -> Option<Self> {
        let ranges = line.strip_prefix("@@ ")?.split_once(" @@")?.0;
        let (old, new) = ranges.split_once(' ')?;
        Some(Self {
            old: length(old.strip_prefix('-')?)?,
            new: length(new.strip_prefix('+')?)?,
        })
    }

    fn take(&mut self, line: &str) -> Option<Mark> {
        if self.old == 0 && self.new == 0 {
            return None;
        }
        match line.as_bytes().first() {
            Some(b'+') if self.new > 0 => {
                self.new -= 1;
                Some(Mark::Added)
            }
            Some(b'-') if self.old > 0 => {
                self.old -= 1;
                Some(Mark::Removed)
            }
            // `\ No newline at end of file` belongs to the line before it.
            Some(b'\\') => None,
            _ => {
                self.old = self.old.saturating_sub(1);
                self.new = self.new.saturating_sub(1);
                Some(Mark::Kept)
            }
        }
    }
}

fn length(range: &str) -> Option<u32> {
    match range.split_once(',') {
        Some((_, length)) => length.parse().ok(),
        None => range.parse::<u32>().ok().map(|_| 1),
    }
}

#[cfg(test)]
mod tests {
    use super::counts;

    #[test]
    fn a_later_files_header_is_not_a_change() {
        let unified = "--- a.rs\n+++ a.rs\n@@ -1,2 +1,3 @@\n one\n-two\n+too\n+three\n\
                       --- b.rs\n+++ b.rs\n@@ -4 +4 @@\n-old\n+new\n";
        assert_eq!(counts(unified), (3, 2));
    }

    #[test]
    fn a_changed_line_that_looks_like_a_header_is_still_counted() {
        let unified = "--- a.md\n+++ a.md\n@@ -1,1 +1,1 @@\n--- rule\n+++ title\n";
        assert_eq!(counts(unified), (1, 1));
    }

    #[test]
    fn a_new_file_and_a_missing_newline_count_as_written() {
        let unified =
            "--- /dev/null\n+++ c.rs\n@@ -0,0 +1,2 @@\n+one\n+two\n\\ No newline at end of file\n";
        assert_eq!(counts(unified), (2, 0));
    }

    #[test]
    fn text_with_no_hunk_counts_nothing() {
        assert_eq!(counts("--- a\n+++ b\n+not a hunk\n"), (0, 0));
    }
}
