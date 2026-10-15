//! The value after the current one: `/model next` walks the pins and
//! `/think next` the efforts, each bound to a chord on its own command.

pub(super) fn after<'a>(values: &'a [String], current: &str) -> Option<&'a str> {
    if values.is_empty() {
        return None;
    }
    let index = values
        .iter()
        .position(|value| value == current)
        .map_or(0, |index| (index + 1) % values.len());
    Some(&values[index])
}
#[cfg(test)]
mod tests {
    #[test]
    fn cycles_in_order_and_starts_at_first_when_absent() {
        let values = vec!["a".into(), "b".into()];
        for (current, expected) in [("a", "b"), ("b", "a"), ("missing", "a")] {
            assert_eq!(super::after(&values, current), Some(expected));
        }
        assert_eq!(super::after(&[], "a"), None);
    }
}
