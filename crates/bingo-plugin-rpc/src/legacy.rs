//! Read obsolete handshake claims before serde discards the unknown field.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::InitializeResult;
    use serde_json::Value;
    #[test]
    fn legacy_fixtures_ignore_the_field_and_name_the_plugin_once() {
        for (fixture, expected) in [(include_str!("../fixtures/legacy/absent.json"), false),
            (include_str!("../fixtures/legacy/true.json"), true)] {
            for protocol in [1, 2] {
                let mut value: Value = serde_json::from_str(fixture).unwrap();
                value["protocol"] = protocol.into();
                let notice = notice("example", &value);
                assert_eq!(notice.is_some(), expected);
                if let Some(notice) = notice {
                    assert_eq!(notice.level, bingo_sdk::Level::Info);
                    assert!(notice.text.contains("example") && notice.text.contains("is an agent"));
                }
                let read: InitializeResult = serde_json::from_value(value).unwrap();
                let rendered = serde_json::to_value(read).unwrap();
                for provider in rendered["providers"].as_array().unwrap() {
                    assert!(provider["endpoint"].get("holdsContext").is_none());
                }
            }
        }
    }
}
