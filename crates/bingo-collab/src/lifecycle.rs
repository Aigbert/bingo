//! The vocabulary shared by listings and surfaces (ADR-0077).
use bingo_sdk::{Driver, SessionState, SessionSummary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Working,
    Asking,
    Idle,
    Stored,
    Unseated,
}

#[derive(Clone, Copy, Debug)]
pub struct Facts {
    pub seated: bool,
    pub log: bool,
    pub open: bool,
    pub asking: bool,
    pub working: bool,
}

pub fn decide(facts: Facts) -> Option<Lifecycle> {
    if !facts.seated {
        Some(Lifecycle::Unseated)
    } else if facts.log {
        None
    } else if !facts.open {
        Some(Lifecycle::Stored)
    } else if facts.asking {
        Some(Lifecycle::Asking)
    } else if facts.working {
        Some(Lifecycle::Working)
    } else {
        Some(Lifecycle::Idle)
    }
}

impl Lifecycle {
    pub fn word(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Asking => "asking",
            Self::Idle => "idle",
            Self::Stored => "stored",
            Self::Unseated => "not seated",
        }
    }

    pub fn of_state(state: &SessionState) -> Option<Self> {
        decide(Facts {
            seated: true,
            log: state.summary.driver == Driver::Log,
            open: !state.closed,
            asking: !state.interactions.is_empty(),
            working: state.busy(),
        })
    }

    pub fn of_summary(summary: &SessionSummary) -> Option<Self> {
        decide(Facts {
            seated: true,
            log: summary.driver == Driver::Log,
            open: true,
            asking: summary.asking,
            working: summary.busy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_order() {
        use Lifecycle::*;
        assert_eq!(
            [Working, Asking, Idle, Stored, Unseated].map(Lifecycle::word),
            ["working", "asking", "idle", "stored", "not seated"]
        );
    }

    #[test]
    fn precedence_including_ties() {
        use Lifecycle::*;
        for (seated, log, open, asking, working, expected) in [
            (false, true, false, true, true, Some(Unseated)),
            (true, true, false, true, true, None),
            (true, false, false, true, true, Some(Stored)),
            (true, false, true, true, true, Some(Asking)),
            (true, false, true, true, false, Some(Asking)),
            (true, false, true, false, true, Some(Working)),
            (true, false, true, false, false, Some(Idle)),
        ] {
            assert_eq!(
                decide(Facts {
                    seated,
                    log,
                    open,
                    asking,
                    working
                }),
                expected
            );
        }
    }
}
