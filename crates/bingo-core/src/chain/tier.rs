//! The authority order, independent of dispatch and I/O.
use bingo_sdk::{Decision, Tier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Observe,
    Rewrite,
    Answer,
    Allow,
}

pub fn tightens(before: &Decision, after: &Decision) -> bool {
    rank(after) >= rank(before)
}
fn rank(decision: &Decision) -> u8 {
    match decision {
        Decision::Allow { .. } => 0,
        Decision::Ask { .. } => 1,
        Decision::Deny { .. } => 2,
    }
}

pub fn admits(tier: Tier, point: &bingo_sdk::catalogue::Entry, act: Act) -> bool {
    match (tier, act) {
        (_, Act::Observe) | (Tier::Full, _) => true,
        (_, Act::Allow) => false,
        (Tier::Tighten, Act::Rewrite | Act::Answer) => [
            bingo_sdk::catalogue::PROMPT_SUBMIT,
            bingo_sdk::catalogue::TOOL_CHECK,
        ]
        .contains(&point),
        (Tier::Rewrite, Act::Rewrite) => point.acts.rewrite,
        (Tier::Rewrite, Act::Answer) => point.acts.answer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::Reason;
    #[test]
    fn deny_is_stricter_than_ask_which_is_stricter_than_allow() {
        let decisions = [
            Decision::Allow {
                reason: Reason::Default,
            },
            Decision::Ask {
                reason: Reason::Default,
                scope: None,
            },
            Decision::Deny {
                reason: Reason::Default,
            },
        ];
        for (before, b) in decisions.iter().enumerate() {
            for (after, a) in decisions.iter().enumerate() {
                assert_eq!(tightens(b, a), after >= before);
            }
        }
    }
    #[test]
    fn each_tier_has_exactly_its_declared_rights() {
        use bingo_sdk::catalogue::{
            PROMPT_COMPOSE as PromptCompose, PROMPT_SUBMIT as PromptSubmit,
            SESSION_COMPACT as SessionCompact, TOOL_CALL as ToolCall, TOOL_CHECK as ToolCheck,
            TURN_STEP as TurnStep, TURN_STOP as TurnStop,
        };
        let points = [
            PromptSubmit,
            ToolCheck,
            ToolCall,
            TurnStep,
            PromptCompose,
            SessionCompact,
            TurnStop,
        ];
        for point in points {
            assert!(admits(Tier::Full, point, Act::Allow));
            assert!(!admits(Tier::Rewrite, point, Act::Allow));
            assert!(!admits(Tier::Tighten, point, Act::Allow));
            assert!(admits(Tier::Tighten, point, Act::Observe));
            assert_eq!(
                admits(Tier::Tighten, point, Act::Rewrite),
                [PromptSubmit, ToolCheck].contains(&point)
            );
            assert_eq!(
                admits(Tier::Rewrite, point, Act::Answer),
                [ToolCall, TurnStep, PromptCompose, SessionCompact].contains(&point)
            );
        }
    }
}
