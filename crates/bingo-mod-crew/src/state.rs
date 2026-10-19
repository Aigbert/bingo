//! What the crew sheet lists (`crewItems`, `crewList`): the teammates by who
//! needs you, then the rooms. Read fresh on every drawing; nothing here is
//! kept between two of them.
use bingo_sdk::SessionId;

/// What a teammate asks of you, in the order the sheet ranks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    Asking,
    Working,
    Idle,
}

/// One teammate of the home session.
#[derive(Clone, Debug, PartialEq)]
pub struct Mate {
    pub id: SessionId,
    pub name: String,
    pub ink: &'static str,
    pub need: Need,
    /// Its live turn's prompt, while it works.
    pub doing: Option<String>,
    /// When its live turn started, in milliseconds.
    pub since: Option<i64>,
    pub role: Option<String>,
}

/// One room of the home session.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: SessionId,
    pub name: String,
    pub purpose: Option<String>,
    pub seated: usize,
    pub closed: bool,
    /// Someone the room called on is writing an answer.
    pub writing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Mate(Mate),
    Room(Place),
}

impl Entry {
    /// The row's identity: the session it walks into.
    pub fn id(&self) -> &SessionId {
        match self {
            Entry::Mate(mate) => &mate.id,
            Entry::Room(place) => &place.id,
        }
    }

    pub fn mate(&self) -> Option<&Mate> {
        match self {
            Entry::Mate(mate) => Some(mate),
            Entry::Room(_) => None,
        }
    }
}

/// The teammates by need, each need in arrival order, then the rooms.
pub fn ordered(mut mates: Vec<Mate>, rooms: Vec<Place>) -> Vec<Entry> {
    mates.sort_by_key(|mate| mate.need);
    mates
        .into_iter()
        .map(Entry::Mate)
        .chain(rooms.into_iter().map(Entry::Room))
        .collect()
}

/// The sheet's fact: who needs you and who works, or that nobody does.
pub fn fact(entries: &[Entry], sep: &str) -> String {
    let count = |need| {
        entries
            .iter()
            .filter_map(Entry::mate)
            .filter(|mate| mate.need == need)
            .count()
    };
    let busy: Vec<String> = [
        (count(Need::Asking), "needs you"),
        (count(Need::Working), "working"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, word)| format!("{n} {word}"))
    .collect();
    if busy.is_empty() {
        "all idle".into()
    } else {
        busy.join(sep)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn mate(name: &str, need: Need) -> Mate {
        Mate {
            id: SessionId::from_raw(format!("ses_{name}")),
            name: name.into(),
            ink: "crew1",
            need,
            doing: None,
            since: None,
            role: None,
        }
    }

    pub fn place(name: &str) -> Place {
        Place {
            id: SessionId::from_raw(format!("ses_{name}")),
            name: format!("#{name}"),
            purpose: None,
            seated: 0,
            closed: false,
            writing: false,
        }
    }

    #[test]
    fn teammates_rank_by_need_in_arrival_order_and_rooms_come_last() {
        let entries = ordered(
            vec![
                mate("ada", Need::Idle),
                mate("scout", Need::Working),
                mate("bo", Need::Asking),
                mate("reviewer", Need::Working),
            ],
            vec![place("design")],
        );
        let ids: Vec<_> = entries.iter().map(|e| e.id().as_str()).collect();
        assert_eq!(
            ids,
            [
                "ses_bo",
                "ses_scout",
                "ses_reviewer",
                "ses_ada",
                "ses_design"
            ]
        );
    }

    #[test]
    fn the_fact_counts_need_and_work_or_says_all_idle() {
        let busy = ordered(
            vec![
                mate("bo", Need::Asking),
                mate("scout", Need::Working),
                mate("ada", Need::Working),
            ],
            vec![],
        );
        assert_eq!(fact(&busy, " · "), "1 needs you · 2 working");
        let quiet = ordered(vec![mate("ada", Need::Idle)], vec![place("design")]);
        assert_eq!(fact(&quiet, " · "), "all idle");
    }
}
