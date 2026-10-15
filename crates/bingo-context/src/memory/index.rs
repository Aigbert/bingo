//! Removing a link removes its line, preserving every other byte.
pub fn unlink(index: &str, file: &str) -> String {
    index
        .split_inclusive('\n')
        .filter(|line| !targets(line, file))
        .collect()
}

fn targets(line: &str, file: &str) -> bool {
    line.match_indices("](").any(|(at, _)| {
        let tail = &line[at + 2..];
        let Some(end) = tail.find(')') else {
            return false;
        };
        let link = tail[..end].trim();
        let target = if let Some(quoted) = link.strip_prefix('<') {
            quoted
                .split_once('>')
                .map(|(target, _)| target)
                .unwrap_or("")
        } else {
            link.split_whitespace().next().unwrap_or("")
        };
        target == file
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_lines_linking_the_exact_file_go() {
        for (input, expected) in [
            (
                "# Memory\n\n- [a](a.md)\n- [b](b.md)\n",
                "# Memory\n\n- [b](b.md)\n",
            ),
            ("[b](b.md) and [a](a.md)\nkeep", "keep"),
            ("[a](a.md \"title\")\r\n\r\nkeep\r\n", "\r\nkeep\r\n"),
            ("[a](<a.md> 'title')\n", ""),
            (
                "[a](aa.md)\n[a](dir/a.md)\na.md\n",
                "[a](aa.md)\n[a](dir/a.md)\na.md\n",
            ),
            ("[a](a.md)", ""),
            ("[a](a.md", "[a](a.md"),
        ] {
            assert_eq!(unlink(input, "a.md"), expected, "{input:?}");
        }
    }
}
