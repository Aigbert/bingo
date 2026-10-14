//! Dated ids collapse only when the same provider also lists their twin.

pub fn alias_of<'a>(id: &str, listed: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    listed
        .into_iter()
        .filter(|prefix| {
            id.strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('-'))
                .is_some_and(date)
        })
        .max_by_key(|prefix| prefix.len())
}

fn date(suffix: &str) -> bool {
    let bytes = suffix.as_bytes();
    match bytes.len() {
        8 => bytes.iter().all(u8::is_ascii_digit),
        10 => bytes.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_a_dated_id_with_a_listed_twin_is_an_alias() {
        for (id, expected) in [
            ("model-20260101", Some("model")),
            ("model-2026-01-01", Some("model")),
            ("model", None),
            ("model-latest", None),
            ("model-2026-1-01", None),
            ("other-20260101", None),
            ("model-2026-01-01-extra", None),
            ("model-２０２６０１０１", None),
        ] {
            assert_eq!(alias_of(id, ["model"]), expected, "{id}");
        }
    }
}
