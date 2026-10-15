//! Which lines only the person may run is the command's to say: a link may
//! describe a command differently, but not lift that from it.
use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

/// Whether a description or a commands catalogue keeps every command's
/// person-only words as the command gave them.
pub(super) fn kept(entry: &catalogue::Entry, before: &Value, after: &Value) -> bool {
    if entry == catalogue::COMMAND_DESCRIBE {
        return after.is_null() || words(before) == words(after);
    }
    if entry == catalogue::PROMPT_AUTOCOMPLETE && before["kind"] == "commands" {
        let given = listed(before);
        return listed(after)
            .iter()
            .all(|(id, words)| given.get(id).is_none_or(|given| given == words));
    }
    true
}

fn words(spec: &Value) -> Option<Vec<String>> {
    serde_json::from_value::<CommandSpec>(spec.clone())
        .ok()
        .map(|spec| spec.args.person_words().to_vec())
}

fn listed(catalog: &Value) -> BTreeMap<String, Option<Vec<String>>> {
    catalog["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| Some((entry["id"].as_str()?.to_string(), words(&entry["meta"]))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plugins(person: Value) -> Value {
        json!({"name":"plugins","hint":"","family":"session",
            "args":{"kind":"words","values":["enable","trust"],"person":person}})
    }

    #[test]
    fn a_description_keeps_the_person_only_words() {
        let given = plugins(json!(["trust"]));
        assert!(kept(catalogue::COMMAND_DESCRIBE, &given, &given));
        assert!(kept(catalogue::COMMAND_DESCRIBE, &given, &Value::Null));
        assert!(!kept(
            catalogue::COMMAND_DESCRIBE,
            &given,
            &plugins(json!([]))
        ));
    }

    #[test]
    fn a_commands_catalogue_keeps_them_for_every_command_it_was_given() {
        let catalog = |person: Value| {
            json!({"kind":"commands","entries":[
                {"id":"plugins","label":"","meta":plugins(person)}
            ]})
        };
        let given = catalog(json!(["trust"]));
        assert!(kept(catalogue::PROMPT_AUTOCOMPLETE, &given, &given));
        assert!(!kept(
            catalogue::PROMPT_AUTOCOMPLETE,
            &given,
            &catalog(json!([]))
        ));
        let models = json!({"kind":"models","entries":[]});
        assert!(kept(catalogue::PROMPT_AUTOCOMPLETE, &models, &models));
    }
}
