//! The next action for a reached call; reaching it belongs to the walk.

use crate::executor::MAX_CONCURRENCY;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Flight {
    pub count: usize,
    pub all_safe: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Decision {
    Allowed { safe: bool },
    Denied,
    Ask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Start,
    Wait,
    Deny,
    Ask,
    Done,
}

pub(crate) fn next(flight: Flight, decision: Option<Decision>) -> Action {
    match decision {
        Some(Decision::Denied) => Action::Deny,
        Some(Decision::Allowed { safe: true })
            if flight.all_safe && flight.count < MAX_CONCURRENCY =>
        {
            Action::Start
        }
        Some(Decision::Allowed { safe: false }) if flight.count == 0 => Action::Start,
        Some(Decision::Ask) if flight.count == 0 => Action::Ask,
        None if flight.count == 0 => Action::Done,
        _ => Action::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reached_call_table() {
        use Action::*;
        let cases = [
            (0, true, [Start, Start, Deny, Ask, Done]),
            (1, true, [Start, Wait, Deny, Wait, Wait]),
            (MAX_CONCURRENCY - 1, true, [Start, Wait, Deny, Wait, Wait]),
            (MAX_CONCURRENCY, true, [Wait, Wait, Deny, Wait, Wait]),
            (1, false, [Wait, Wait, Deny, Wait, Wait]),
            (MAX_CONCURRENCY, false, [Wait, Wait, Deny, Wait, Wait]),
        ];
        let decisions = [
            Some(Decision::Allowed { safe: true }),
            Some(Decision::Allowed { safe: false }),
            Some(Decision::Denied),
            Some(Decision::Ask),
            None,
        ];
        for (count, all_safe, expected) in cases {
            for (decision, action) in decisions.into_iter().zip(expected) {
                assert_eq!(
                    next(Flight { count, all_safe }, decision),
                    action,
                    "count={count}, all_safe={all_safe}, decision={decision:?}"
                );
            }
        }
        assert_eq!(MAX_CONCURRENCY, 10);
    }
}
