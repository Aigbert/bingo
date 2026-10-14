//! A source reports the changes made by an explicit rescan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginReload {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub reloaded: Vec<String>,
    pub errors: Vec<String>,
}
impl PluginReload {
    pub fn append(&mut self, other: Self) {
        self.added.extend(other.added);
        self.removed.extend(other.removed);
        self.reloaded.extend(other.reloaded);
        self.errors.extend(other.errors);
    }
    pub fn message(&self) -> String {
        let mut message =
            if self.added.is_empty() && self.removed.is_empty() && self.reloaded.is_empty() {
                "nothing changed".into()
            } else {
                let names = if self.added.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", self.added.join(", "))
                };
                format!(
                    "reloaded: {} added{names}, {} removed, {} reloaded",
                    self.added.len(),
                    self.removed.len(),
                    self.reloaded.len()
                )
            };
        if !self.errors.is_empty() {
            message.push_str("; errors: ");
            message.push_str(
                &self
                    .errors
                    .iter()
                    .map(|error| error.split_whitespace().collect::<Vec<_>>().join(" "))
                    .collect::<Vec<_>>()
                    .join("; "),
            );
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn summaries_name_additions_and_keep_errors_on_one_line() {
        assert_eq!(PluginReload::default().message(), "nothing changed");
        let mut report = PluginReload {
            added: vec!["bingo-mascot".into()],
            reloaded: vec!["a".into(), "b".into()],
            ..Default::default()
        };
        assert_eq!(
            report.message(),
            "reloaded: 1 added (bingo-mascot), 0 removed, 2 reloaded"
        );
        report.errors.push("broken:\nsyntax error".into());
        assert!(report.message().ends_with("; errors: broken: syntax error"));
    }
}
