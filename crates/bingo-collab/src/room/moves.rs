//! What changed in a room, as the room reads it: the line its roster verbs
//! post under the caller's name, and the same line read back. The rooms plugin
//! writes it and a surface reads it, so its spelling lives here once.

/// One change to a room: who came in, who went out, or its end and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Move {
    Seated(Vec<String>),
    Unseated(Vec<String>),
    Closed { title: String, why: Option<String> },
}

const SEATED: &str = "seated";
const UNSEATED: &str = "unseated";
const CLOSED: &str = "closed";

impl Move {
    /// The post `by` signs for it.
    pub fn said(&self, by: &str) -> String {
        match self {
            Move::Seated(names) => format!("{by} {SEATED} {}", names.join(", ")),
            Move::Unseated(names) => format!("{by} {UNSEATED} {}", names.join(", ")),
            Move::Closed { title, why: None } => format!("{by} {CLOSED} {title}"),
            Move::Closed {
                title,
                why: Some(why),
            } => format!("{by} {CLOSED} {title}: {why}"),
        }
    }

    /// The change a post says, when `by` signed it and it is one.
    pub fn read(by: &str, text: &str) -> Option<Move> {
        let rest = text.strip_prefix(by)?.strip_prefix(' ')?;
        let (verb, rest) = rest.split_once(' ')?;
        match verb {
            SEATED => Some(Move::Seated(names(rest))),
            UNSEATED => Some(Move::Unseated(names(rest))),
            CLOSED => Some(closed(rest)),
            _ => None,
        }
    }
}

fn names(list: &str) -> Vec<String> {
    list.split(", ").map(str::to_string).collect()
}

/// A room's title has no space and no colon, so the first `: ` ends it.
fn closed(rest: &str) -> Move {
    match rest.split_once(": ") {
        Some((title, why)) => Move::Closed {
            title: title.into(),
            why: Some(why.into()),
        },
        None => Move::Closed {
            title: rest.into(),
            why: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_move_reads_back_as_it_was_said() {
        for change in [
            Move::Seated(vec!["scout".into(), "ada".into()]),
            Move::Unseated(vec!["scout".into()]),
            Move::Closed {
                title: "#design".into(),
                why: Some("we chose: keep the queue".into()),
            },
            Move::Closed {
                title: "#design".into(),
                why: None,
            },
        ] {
            assert_eq!(Move::read("parent", &change.said("parent")), Some(change));
        }
    }

    #[test]
    fn words_that_are_not_a_move_are_a_post() {
        assert_eq!(Move::read("parent", "then let us ship it"), None);
        assert_eq!(Move::read("scout", "parent seated ada"), None);
        assert_eq!(Move::read("parent", "parent"), None);
    }
}
