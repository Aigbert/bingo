//! The update setting claimed by the terminal surface.

use schemars::JsonSchema;
use serde::Deserialize;

/// The top-level key.
pub const SETTING: &str = "update";

/// `update` is claimed by the surface that says what the check found, so a
/// misspelling of *it* is reported as an unknown setting like any other. A
/// misspelling of what is inside it leaves the check where it was — on —
/// which is the safe way for this one setting to be wrong.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Update {
    /// Whether a start asks whether a newer release is out. On unless it is
    /// turned off: the check is once a day, off the start's own thread, and
    /// silent whenever it cannot answer.
    #[serde(default = "on")]
    pub check: bool,
}

impl Default for Update {
    fn default() -> Self {
        Self { check: on() }
    }
}

fn on() -> bool {
    true
}
