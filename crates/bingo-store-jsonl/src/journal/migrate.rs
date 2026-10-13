//! Each step upgrades one journal version before the live frame is decoded.

use serde_json::Value;

pub(super) const STEPS: &[fn(Value) -> Value] = &[];

pub(super) fn upgrade(value: Value, version: u32, steps: &[fn(Value) -> Value]) -> Value {
    steps
        .iter()
        .skip(version.saturating_sub(1) as usize)
        .fold(value, |value, step| step(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_version_has_exactly_one_step_to_its_successor() {
        assert_eq!(STEPS.len(), super::super::VERSION as usize - 1);
    }

    #[test]
    fn steps_chain_from_the_lines_version_before_decoding() {
        fn rename(mut value: Value) -> Value {
            value["new"] = value.as_object_mut().unwrap().remove("old").unwrap();
            value
        }
        fn wrap(value: Value) -> Value {
            json!({"wrapped": value})
        }
        let steps: &[fn(Value) -> Value] = &[rename, wrap];
        assert_eq!(
            upgrade(json!({"old": 42}), 1, steps),
            json!({"wrapped": {"new": 42}})
        );
        assert_eq!(
            upgrade(json!({"new": 42}), 2, steps),
            json!({"wrapped": {"new": 42}})
        );
        let current = json!({"wrapped": {"new": 42}});
        assert_eq!(upgrade(current.clone(), 3, steps), current);
    }
}
