//! The last thing a failed server said on its stderr, read bounded and
//! redacted, so a card can say it without a person opening the log.
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Enough to hold the line a person reads; never the whole log.
const TAIL: u64 = 4096;
/// Longer than any card is wide: the card cuts it where it must.
const LINE: usize = 240;

/// The last non-blank line of a log, with tokens and control bytes cut out.
pub fn last_line(path: &Path) -> Option<String> {
    let tail = tail(path).ok()?;
    let line = tail
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let shown = bingo_services::mcp::redacted_line(&printable(line))
        .chars()
        .take(LINE)
        .collect::<String>();
    (!shown.is_empty()).then_some(shown)
}

/// A line without the colour escapes and control bytes a terminal would act on.
fn printable(line: &str) -> String {
    let mut shown = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            skip_escape(&mut chars);
        } else if !c.is_control() {
            shown.push(c);
        }
    }
    shown
}

/// A CSI sequence ends at its final byte; any other escape is two characters.
fn skip_escape(chars: &mut std::str::Chars<'_>) {
    if chars.next() == Some('[') {
        for c in chars.by_ref() {
            if ('@'..='~').contains(&c) {
                break;
            }
        }
    }
}

fn tail(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL)))?;
    let mut bytes = Vec::new();
    file.take(TAIL).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_line_is_read_from_the_tail_and_redacted() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("mcp-files.log");
        let mut text = "x".repeat(3 * TAIL as usize);
        text.push_str("\nstarting\n\x1b[31mfatal: API_KEY=s3cret --token s3cret\x1b[0m\n\n");
        std::fs::write(&log, text).unwrap();
        assert_eq!(
            last_line(&log).as_deref(),
            Some("fatal: API_KEY=… --token …")
        );
        assert_eq!(last_line(&dir.path().join("absent.log")), None);
    }
}
