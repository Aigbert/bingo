use crate::rows::{Row, Target, matched};
use bingo_sdk::{ArgSpec, CatalogEntry, CommandSpec};
use std::collections::BTreeMap;

pub struct Arguments {
    pub title: String,
    pub rows: Vec<Row>,
    pub selected: usize,
}
pub fn arguments(
    line: &str,
    commands: &[CatalogEntry],
    catalogs: &BTreeMap<String, Vec<CatalogEntry>>,
) -> Option<Arguments> {
    let (name, text) = line.strip_prefix('/')?.split_once(' ')?;
    let entry = commands.iter().find(|entry| {
        entry.id == name
            || entry.meta["aliases"]
                .as_array()
                .is_some_and(|aliases| aliases.iter().any(|alias| alias == name))
    })?;
    let spec: CommandSpec = serde_json::from_value(entry.meta.clone()).ok()?;
    let (args, prefix, query) = resolve(&spec.args, text, format!("/{name} "));
    let current = entry.meta["now"].as_str().unwrap_or_default();
    let rows = option_rows(&spec.args, text, &format!("/{name} "), catalogs, current)
        .unwrap_or_else(|| offered(args, &prefix, query, catalogs, current));
    let selected = rows.iter().position(|row| row.now == "now").unwrap_or(0);
    Some(Arguments {
        title: format!("/{name}  {}", spec.hint),
        rows,
        selected,
    })
}
fn resolve<'a>(args: &'a ArgSpec, text: &'a str, prefix: String) -> (&'a ArgSpec, String, &'a str) {
    match args {
        ArgSpec::Options { args, .. } => resolve(args, text, prefix),
        ArgSpec::Words {
            values,
            alone,
            then: Some(next),
        } => {
            if let Some((word, rest)) = text.split_once(' ')
                && values.iter().any(|v| v == word)
                && !alone.iter().any(|v| v == word)
            {
                return resolve(next, rest, format!("{prefix}{word} "));
            }
            (args, prefix, text)
        }
        _ => (args, prefix, text),
    }
}
fn offered(
    args: &ArgSpec,
    prefix: &str,
    query: &str,
    catalogs: &BTreeMap<String, Vec<CatalogEntry>>,
    current: &str,
) -> Vec<Row> {
    match args {
        ArgSpec::Words {
            values,
            alone,
            then,
        } => values
            .iter()
            .filter_map(|value| {
                choice(
                    value,
                    value,
                    prefix,
                    query,
                    current,
                    then.is_some() && !alone.contains(value),
                )
            })
            .collect(),
        ArgSpec::Catalog { source } => catalogs
            .get(source)
            .into_iter()
            .flatten()
            .filter_map(|entry| choice(&entry.id, &entry.label, prefix, query, current, false))
            .collect(),
        ArgSpec::Free { hint } => vec![Row {
            label: if query.is_empty() {
                hint.clone()
            } else {
                preview(prefix, query)
            },
            hint: String::new(),
            now: current.into(),
            alert: None,
            marks: vec![],
            hint_marks: vec![],
            target: Target::Run(format!("{prefix}{query}").trim_end().into()),
        }],
        ArgSpec::None => vec![Row {
            label: format!("⏎ runs {}", format!("{prefix}{query}").trim_end()),
            hint: String::new(),
            now: String::new(),
            alert: None,
            marks: vec![],
            hint_marks: vec![],
            target: Target::Run(format!("{prefix}{query}").trim_end().into()),
        }],
        ArgSpec::Options { args, .. } => offered(args, prefix, query, catalogs, current),
    }
}
fn choice(
    value: &str,
    label: &str,
    prefix: &str,
    query: &str,
    current: &str,
    more: bool,
) -> Option<Row> {
    let marks = matched(label, query).or_else(|| matched(value, query))?;
    let line = format!("{prefix}{value}");
    Some(Row {
        label: label.into(),
        hint: String::new(),
        now: if value == current || current.ends_with(&format!("/{value}")) {
            "now".into()
        } else {
            String::new()
        },
        alert: None,
        marks,
        hint_marks: vec![],
        target: if more {
            Target::Edit(format!("{line} "))
        } else {
            Target::Run(line)
        },
    })
}

fn option_rows(
    args: &ArgSpec,
    text: &str,
    prefix: &str,
    catalogs: &BTreeMap<String, Vec<CatalogEntry>>,
    current: &str,
) -> Option<Vec<Row>> {
    let ArgSpec::Options { args, flags } = args else {
        return None;
    };
    let (head, query) = text.rsplit_once(' ')?;
    if !offered(args, prefix, head, catalogs, current)
        .iter()
        .any(|row| matches!(&row.target,Target::Run(line) if line==&format!("{prefix}{head}")))
    {
        return None;
    }
    Some(
        flags
            .iter()
            .filter_map(|flag| choice(flag, flag, &format!("{prefix}{head} "), query, "", false))
            .collect(),
    )
}

fn preview(prefix: &str, query: &str) -> String {
    match prefix {
        "/rename " => format!("⏎ names this session “{query}”"),
        "/compact " => format!("⏎ compacts, keeping {query}"),
        _ => format!("⏎ runs {prefix}{query}"),
    }
}

pub fn parent(line: &str, commands: &[CatalogEntry]) -> Option<String> {
    let (name, text) = line.strip_prefix('/')?.split_once(' ')?;
    let entry = commands.iter().find(|entry| {
        entry.id == name
            || entry.meta["aliases"]
                .as_array()
                .is_some_and(|aliases| aliases.iter().any(|alias| alias == name))
    })?;
    let spec: CommandSpec = serde_json::from_value(entry.meta.clone()).ok()?;
    let (_, prefix, _) = resolve(&spec.args, text, format!("/{name} "));
    Some(
        prefix
            .trim_end()
            .rsplit_once(' ')
            .map(|(head, _)| format!("{head} "))
            .unwrap_or_else(|| "/".into()),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn words_options_catalog_free_and_none_have_one_argument_path() {
        let catalogs = BTreeMap::new();
        for args in [
            ArgSpec::None,
            ArgSpec::Free {
                hint: "title".into(),
            },
            ArgSpec::Words {
                values: vec!["low".into()],
                alone: vec![],
                then: None,
            },
            ArgSpec::Options {
                args: Box::new(ArgSpec::Words {
                    values: vec!["low".into()],
                    alone: vec![],
                    then: None,
                }),
                flags: vec!["--keep".into()],
            },
        ] {
            assert_eq!(offered(&args, "/think ", "", &catalogs, "low").len(), 1);
        }
        assert!(
            offered(
                &ArgSpec::Catalog {
                    source: "models".into()
                },
                "/model ",
                "",
                &catalogs,
                ""
            )
            .is_empty()
        );
        let args = ArgSpec::Words {
            values: vec!["enable".into(), "reload".into()],
            alone: vec!["reload".into()],
            then: Some(Box::new(ArgSpec::Free {
                hint: "plugin".into(),
            })),
        };
        let (next, prefix, query) = resolve(&args, "enable abc", "/plugins ".into());
        assert!(matches!(next, ArgSpec::Free { .. }));
        assert_eq!(prefix, "/plugins enable ");
        assert_eq!(query, "abc");
        assert!(matches!(
            offered(&args, "/plugins ", "reload", &catalogs, "")[0].target,
            Target::Run(_)
        ));
    }
    #[test]
    fn arguments_select_now_offer_flags_and_escape_a_whole_level() {
        let catalogs = BTreeMap::from([(
            "models".into(),
            vec![CatalogEntry {
                id: "fake/large".into(),
                label: "Large".into(),
                meta: serde_json::Value::Null,
            }],
        )]);
        let commands = vec![
            CatalogEntry {
                id: "model".into(),
                label: "model".into(),
                meta: serde_json::json!({"name":"model","hint":"choose","family":"session","now":"fake/large","args":{"kind":"options","args":{"kind":"catalog","source":"models"},"flags":["--keep"]}}),
            },
            CatalogEntry {
                id: "rename".into(),
                label: "rename".into(),
                meta: serde_json::json!({"name":"rename","hint":"name","family":"session","args":{"kind":"free","hint":"title"}}),
            },
        ];
        let offered = arguments("/model ", &commands, &catalogs).unwrap();
        assert_eq!(offered.rows[offered.selected].now, "now");
        assert_eq!(
            offered.rows[0].target,
            Target::Run("/model fake/large".into())
        );
        let flags = arguments("/model fake/large ", &commands, &catalogs).unwrap();
        assert_eq!(
            flags.rows[0].target,
            Target::Run("/model fake/large --keep".into())
        );
        assert_eq!(parent("/rename several words", &commands), Some("/".into()));
        assert_eq!(parent("/", &commands), None);
    }
}
