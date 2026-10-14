//! Resolve addresses and the origins a configured key is allowed to reach.
use bingo_sdk::plugin::ConfigSources;
use serde_json::Value;

pub fn resolve(setting: Option<&str>, default: &str, environment: Option<&str>) -> String {
    environment
        .and_then(nonblank)
        .or_else(|| setting.and_then(nonblank))
        .unwrap_or(default)
        .into()
}

fn nonblank(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

pub fn configured(
    sources: Option<&ConfigSources>,
    setting: &str,
    default: &str,
    environment: Option<&str>,
    from_environment: bool,
) -> String {
    if from_environment {
        return resolve(None, default, environment);
    }
    let Some(sources) = sources else {
        return resolve(None, default, environment);
    };
    let mut path: Vec<_> = setting.split('.').collect();
    let key_layer = sources
        .layers
        .iter()
        .rposition(|layer| value(&layer.values, &path).is_some());
    path.pop();
    path.push("baseUrl");
    let address = sources
        .layers
        .iter()
        .enumerate()
        .rev()
        .filter(|(index, layer)| !layer.repository || key_layer.is_some_and(|key| *index <= key))
        .find_map(|(_, layer)| value(&layer.values, &path).and_then(Value::as_str));
    resolve(address, default, environment)
}

fn value<'a>(values: &'a serde_json::Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(values.get(*first)?, |value, key| value.get(*key))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layer(source: &str, repository: bool, values: Value) -> bingo_sdk::plugin::ConfigLayer {
        bingo_sdk::plugin::ConfigLayer {
            source: source.into(),
            repository,
            user: !repository,
            values: values.as_object().unwrap().clone(),
        }
    }

    #[test]
    fn settings_keys_use_only_their_own_or_more_trusted_address() {
        use serde_json::json;
        let mut sources = ConfigSources::default();
        sources.layers.push(layer(
            "user",
            false,
            json!({"family":{"baseUrl":"https://user", "apiKey":"dummy"}}),
        ));
        sources.layers.push(layer(
            "project",
            true,
            json!({"family":{"baseUrl":"https://project"}}),
        ));
        let resolve = |sources: &ConfigSources| {
            configured(
                Some(sources),
                "family.apiKey",
                "https://default",
                None,
                false,
            )
        };
        assert_eq!(resolve(&sources), "https://user");
        sources.layers[1].values["family"]["apiKey"] = json!("project-key");
        assert_eq!(resolve(&sources), "https://project");
        sources.layers.push(layer(
            "local",
            true,
            json!({"family":{"baseUrl":"https://local"}}),
        ));
        assert_eq!(resolve(&sources), "https://project");
        sources.layers.push(layer(
            "explicit",
            false,
            json!({"family":{"baseUrl":"https://explicit"}}),
        ));
        assert_eq!(resolve(&sources), "https://explicit");
        assert_eq!(
            configured(
                Some(&sources),
                "family.apiKey",
                "https://default",
                None,
                true
            ),
            "https://default"
        );
        assert_eq!(
            configured(
                Some(&sources),
                "family.apiKey",
                "https://default",
                Some("https://env"),
                true
            ),
            "https://env"
        );
    }

    #[test]
    fn blank_addresses_are_unset_and_environment_wins() {
        assert_eq!(
            resolve(Some(" "), "https://default", None),
            "https://default"
        );
        assert_eq!(
            resolve(Some("https://setting"), "https://default", Some(" ")),
            "https://setting"
        );
        assert_eq!(
            resolve(
                Some("https://setting"),
                "https://default",
                Some("https://env")
            ),
            "https://env"
        );
    }
}
