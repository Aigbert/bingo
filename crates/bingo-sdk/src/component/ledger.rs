//! A step stays one row throughout its turn (ADR-0097).
use super::*;
use crate::Upshot;

/// Where a step's row starts: marks sit at col 1, steps under the words.
pub const STEP_INDENT: &str = "   ";

/// What a step is doing, as far as the turn has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Running,
    /// Waits for a person's answer.
    Asked,
    Done,
    Failed,
    /// Was in flight when the person stopped the turn.
    Stopped,
    /// The person refused the call before it ran.
    Skipped,
}

/// The catalogue's baseline: only a trusted read-only tool is a lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Lookup,
    Change,
}

impl Tier {
    /// Unknown or untrusted traits cannot make a row a lookup.
    pub fn of(traits: Option<&crate::ToolTraits>) -> Self {
        match traits {
            Some(traits) if traits.trusted && traits.read_only => Self::Lookup,
            _ => Self::Change,
        }
    }
}

/// One tool call of a turn, in the words a row shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// The declared one-cell mark, resolved for the surface character set.
    pub mark: char,
    /// How to render the target, absent when the call has no target.
    pub reads: Option<crate::Reads>,
    /// The catalogue baseline: only trusted read-only tools are lookups.
    pub tier: Tier,
    /// What the step does, one lowercase word: `read`, `run`.
    pub verb: String,
    /// What it does it to, shortened for one row.
    pub target: String,
    pub state: StepState,
    /// What the tool said the call came to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upshot: Option<Upshot>,
    /// What the person answered: a question response or `changed by you`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Upshot>,
    /// Why the step ran without asking, such as `accept edits`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// Elapsed milliseconds; the surface truncates these to whole seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    /// When the result arrived; the drawing declares its landing motion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed_at: Option<i64>,
    /// The session this step started or addressed, when it has a voice of
    /// its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teammate: Option<Teammate>,
}

/// The session a step started or addressed, in the words its row needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Teammate {
    pub name: String,
    /// The theme token the session is drawn in.
    pub ink: String,
    /// When a post from that session first followed the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_at: Option<i64>,
}

/// The only state mark supplied by a surface; tool marks belong to tools.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Marks {
    pub asked: String,
}

/// The `Ledger` site, asked for one step at a time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LedgerProps {
    pub steps: Vec<Step>,
    pub marks: Marks,
}

mod draw;
