use bingo_sdk::transcript::Turn;
use bingo_services::checkpoints::{Fate, Touched};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Both,
    Conversation,
    Files,
}
impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Conversation => "conversation",
            Self::Files => "files",
        }
    }
    pub fn step(self, by: isize) -> Self {
        let at = match self {
            Self::Both => 0,
            Self::Conversation => 1,
            Self::Files => 2,
        };
        [Self::Both, Self::Conversation, Self::Files][(at + by).rem_euclid(3) as usize]
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Sheet {
    pub title: String,
    pub rows: Vec<String>,
    pub line: String,
}
pub fn sheet(turns: &[Turn], touched: &Touched, mode: Mode) -> Option<Sheet> {
    let first = turns.first()?;
    let items = turns.last()?.last - first.first + 1;
    let mut rows = vec![format!("undoes {} turns · {items} items", turns.len())];
    if mode == Mode::Files {
        rows[0] = "no turns are undone".into();
    }
    if mode == Mode::Conversation {
        rows.push("files are kept".into());
    } else if touched.files.is_empty() {
        rows.push("no tracked files".into());
    }
    for file in touched.files.iter().filter(|_| mode != Mode::Conversation) {
        let fate = match file.fate {
            Fate::Restored => "restored",
            Fate::Removed => "removed",
            Fate::Skipped => "skipped",
        };
        let why = if file.fate == Fate::Skipped {
            " (too large or not a file)"
        } else {
            ""
        };
        let changed = if file.changed {
            " · changed since the turn"
        } else {
            ""
        };
        rows.push(format!("{fate:8} {}{why}{changed}", file.path.display()));
    }
    if touched.shell_calls > 0 {
        rows.push(format!(
            "! files a shell command wrote are not tracked ({} Bash calls)",
            touched.shell_calls
        ));
    }
    rows.push(
        match mode {
            Mode::Both => "restore files, then rewind the conversation",
            Mode::Conversation => "rewind the conversation; files are kept",
            Mode::Files => "restore files; conversation is kept",
        }
        .into(),
    );
    Some(Sheet {
        title: format!("rewind to {}", first.label()),
        rows,
        line: format!("/rewind {} {}", first.id, mode.name()),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sheet_names_each_mode_and_untracked_changes() {
        let turns = vec![Turn {
            id: bingo_sdk::TurnId::from_raw("t"),
            asked: None,
            first: 0,
            last: 3,
        }];
        for mode in [Mode::Both, Mode::Conversation, Mode::Files] {
            for calls in [0, 2] {
                let touched = Touched {
                    files: vec![],
                    shell_calls: calls,
                };
                let sheet = sheet(&turns, &touched, mode).unwrap();
                assert_eq!(sheet.line, format!("/rewind t {}", mode.name()));
                assert!(sheet.rows.iter().any(|r| r
                    == if mode == Mode::Conversation {
                        "files are kept"
                    } else {
                        "no tracked files"
                    }));
                assert_eq!(sheet.rows.iter().any(|r| r.contains("Bash")), calls > 0);
            }
        }
        let touched = Touched {
            files: vec![bingo_services::checkpoints::File {
                path: "big".into(),
                fate: Fate::Skipped,
                changed: true,
            }],
            shell_calls: 0,
        };
        assert!(sheet(&turns, &touched, Mode::Both).unwrap().rows[1].contains("skipped"));
        assert!(
            sheet(&turns, &touched, Mode::Both).unwrap().rows[1].contains("changed since the turn")
        );
        let conversation = sheet(&turns, &touched, Mode::Conversation).unwrap();
        assert!(!conversation.rows.iter().any(|row| row.contains("big")));
        assert_eq!(
            sheet(&turns, &touched, Mode::Files).unwrap().rows[0],
            "no turns are undone"
        );
        assert!(sheet(&[], &touched, Mode::Both).is_none());
    }
}
