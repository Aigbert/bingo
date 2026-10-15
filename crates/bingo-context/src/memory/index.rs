//! Removing a link removes its line, preserving every other byte.
pub fn unlink(index: &str, file: &str) -> String {
    index
        .split_inclusive('\n')
        .filter(|line| !targets(line, file))
        .collect()
}

pub(super) fn targets(line: &str, file: &str) -> bool {
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
        normalized(target).as_deref() == Some(file)
    })
}

fn normalized(target: &str) -> Option<String> {
    let target = target.split('#').next()?;
    let mut decoded = Vec::new();
    let mut bytes = target.bytes();
    while let Some(byte) = bytes.next() {
        decoded.push(if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            (high * 16 + low) as u8
        } else {
            byte
        });
    }
    let decoded = String::from_utf8(decoded).ok()?;
    Some(decoded.strip_prefix("./").unwrap_or(&decoded).to_owned())
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
            (
                "[a](./a.md#part)\n[a](%61%2Emd)\n[a](%2E%2Fa.md#part)\n",
                "",
            ),
            (
                "[a](A.md)\n[a](a%23part.md)\n[a](%zz.md)",
                "[a](A.md)\n[a](a%23part.md)\n[a](%zz.md)",
            ),
            ("[a](a.md", "[a](a.md"),
        ] {
            assert_eq!(unlink(input, "a.md"), expected, "{input:?}");
        }
    }
}
