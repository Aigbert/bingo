//! A background reply owed to the parent, kept in the parent's journal.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_obligation_has_one_persisted_shape() {
        let expected: serde_json::Value = serde_json::from_str(include_str!("../fixtures/report.json")).unwrap();
        let obligation: Obligation = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(obligation.kind(), "report:ses_scout");
        assert_eq!(serde_json::to_value(obligation).unwrap(), expected);
    }
}
