//! Frontmatter is data; only the document body is prose. The pager renders
//! what it is given as Markdown, so the files this panel reads arrive fenced.
pub fn document(text: &str) -> String {
    let mut lines = text.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("---") {
        return text.into();
    }
    let mut end = text.find('\n').map(|at| at + 1).unwrap_or(text.len());
    for line in lines {
        end += line.len();
        if matches!(line.trim_end(), "---" | "...") {
            let (front, body) = text.split_at(end);
            let longest = front.split(|c| c != '~').map(str::len).max().unwrap_or(0);
            let fence = "~".repeat((longest + 1).max(3));
            return format!("{fence}yaml\n{front}\n{fence}\n\n{body}");
        }
    }
    text.into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontmatter_is_fenced_without_changing_body_markdown() {
        let source = "---\r\nname: rule\r\ntype: feedback\r\n---\r\n**body**";
        assert_eq!(
            document(source),
            "~~~yaml\n---\r\nname: rule\r\ntype: feedback\r\n---\r\n\n~~~\n\n**body**"
        );
        assert_eq!(document("ordinary **body**"), "ordinary **body**");
        assert_eq!(document("---\nunfinished"), "---\nunfinished");
    }
}
