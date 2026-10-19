//! The words a session rule is read in, so `Yes, allow … for this session`
//! names what it allows rather than the rule's grammar.

/// What a session rule allows: the words that lead into it, and the part a
/// person recognises, drawn as `code` while its answer is picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub lead: Option<&'static str>,
    pub code: String,
}

/// A command rule reads as its command, a path rule as the directory's last
/// two parts, a domain rule as the host; any other rule reads as written.
/// Whether the rule names a command is the caller's to say: the surface
/// knows a shell call by its preview, never by the tool's name.
pub fn words(rule: &str, command: bool) -> Scope {
    let plain = |code: &str| Scope {
        lead: None,
        code: code.to_owned(),
    };
    let Some(content) = parts(rule) else {
        return plain(rule);
    };
    if command {
        return plain(content.strip_suffix(":*").unwrap_or(content).trim());
    }
    if let Some(host) = content.strip_prefix("domain:") {
        return Scope {
            lead: Some("fetches from"),
            code: host.to_owned(),
        };
    }
    if let Some(separator) = directory(content) {
        return Scope {
            lead: Some("edits in"),
            code: tail(content, separator),
        };
    }
    plain(rule)
}

/// What `Tool(content)` holds between its parentheses.
fn parts(rule: &str) -> Option<&str> {
    let (_, rest) = rule.split_once('(')?;
    rest.strip_suffix(')')
}

/// The separator a directory rule ends with, which is what makes it one.
fn directory(content: &str) -> Option<char> {
    content.chars().last().filter(|c| matches!(c, '/' | '\\'))
}

/// The last two directories, keeping the separator the rule ends with.
fn tail(path: &str, separator: char) -> String {
    let parts: Vec<&str> = path.split(separator).collect();
    parts[parts.len().saturating_sub(3)..].join(&separator.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(rule: &str) -> (Option<&'static str>, String) {
        let scope = words(rule, rule.starts_with("Bash("));
        (scope.lead, scope.code)
    }

    #[test]
    fn a_command_rule_reads_as_its_command() {
        assert_eq!(read("Bash(cargo test:*)"), (None, "cargo test".into()));
        assert_eq!(read("Bash(cargo:*)"), (None, "cargo".into()));
        assert_eq!(read("Bash(git status)"), (None, "git status".into()));
    }

    #[test]
    fn a_path_rule_reads_as_edits_in_its_last_two_directories() {
        assert_eq!(
            read("Edit(/work/bingo/crates/bingo-core/src/session/)"),
            (Some("edits in"), "src/session/".into())
        );
        assert_eq!(read("Write(/work/)"), (Some("edits in"), "/work/".into()));
        assert_eq!(
            read(r"Edit(C:\work\src\session\)"),
            (Some("edits in"), r"src\session\".into())
        );
    }

    #[test]
    fn a_domain_rule_reads_as_fetches_from_its_host() {
        assert_eq!(
            read("WebFetch(domain:docs.rs)"),
            (Some("fetches from"), "docs.rs".into())
        );
    }

    #[test]
    fn any_other_rule_reads_as_written() {
        for rule in ["Write", "Skill(review-pr)", "Edit(src/lib.rs)", "odd(", ""] {
            assert_eq!(read(rule), (None, rule.to_owned()), "{rule:?}");
        }
    }
}
