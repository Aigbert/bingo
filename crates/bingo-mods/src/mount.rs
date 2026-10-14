//! The headless terminal client uses the same render and action doors.
use bingo_sdk::*;
use serde_json::Value;
pub fn action(_tree: &View, _key: &str, _event: &str, _value: Value) -> Result<Action, String> {
    Err("no matching handler".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn interaction_keeps_the_minted_action_and_sets_only_its_value() {
        let tree: View = serde_json::from_value(json!({"kind":"Input","key":"name","onInput":{"name":"counter:ui","args":{"handler":"h","event":"input"}}})).unwrap();
        let fired = action(&tree,"name","input",json!("Ada")).unwrap();
        assert_eq!(fired.name,"counter:ui");
        assert_eq!(fired.args,json!({"handler":"h","event":"input","value":"Ada"}));
        assert!(action(&tree,"missing","input",json!("Ada")).is_err());
        assert!(action(&tree,"name","press",Value::Null).is_err());
    }
}
