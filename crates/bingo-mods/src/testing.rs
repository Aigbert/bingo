//! One report per test, suitable for a text line or NDJSON.
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub file: String,
    pub test: String,
    pub outcome: Outcome,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Outcome {
    Pass,
    Fail { message: String, at: String },
    Skip,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_round_trip_as_single_json_lines() {
        for outcome in [
            Outcome::Pass,
            Outcome::Fail {
                message: "expected 2\nreceived 1".into(),
                at: "register.test.ts:4".into(),
            },
            Outcome::Skip,
        ] {
            let report = Report {
                file: "register.test.ts".into(),
                test: "counts".into(),
                outcome,
            };
            let line = serde_json::to_string(&report).unwrap();
            assert_eq!(line.lines().count(), 1);
            assert_eq!(serde_json::from_str::<Report>(&line).unwrap(), report);
        }
    }
}
