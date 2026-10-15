use crate::rows::{Row, Target, matched};
use bingo_sdk::{ArgSpec, CatalogEntry, CommandSpec};
use std::collections::BTreeMap;

pub struct Arguments {
    pub title: String,
    pub rows: Vec<Row>,
    pub selected: usize,
}
/// What every row of one argument level is offered from.
struct Offer<'a> {
    catalogs: &'a BTreeMap<String, Vec<CatalogEntry>>,
    current: &'a str,
    /// The level's lines are ones only the person may run, so a row shows
    /// the argument it runs and whatever names it only as secondary text.
    guarded: bool,
}
fn named<'a>(name: &str, commands: &'a [CatalogEntry]) -> Option<(&'a CatalogEntry, CommandSpec)> {
    let entry = commands.iter().find(|entry| {
        entry.id == name
            || entry.meta["aliases"]
                .as_array()
                .is_some_and(|aliases| aliases.iter().any(|alias| alias == name))
    })?;
    Some((entry, serde_json::from_value(entry.meta.clone()).ok()?))
}
/// Whether `line` is one only the person may run.
pub fn person_only(line: &str, commands: &[CatalogEntry]) -> bool {
    line.strip_prefix('/')
        .and_then(|rest| {
            let (name, args) = bingo_sdk::split_verb(rest);
            named(name, commands).map(|(_, spec)| spec.person_only(args))
        })
        .unwrap_or(false)
}
pub fn arguments(
    line: &str,
    commands: &[CatalogEntry],
    catalogs: &BTreeMap<String, Vec<CatalogEntry>>,
) -> Option<Arguments> {
    let (name, text) = line.strip_prefix('/')?.split_once(' ')?;
    let (entry, spec) = named(name, commands)?;
    let (args, prefix, query) = resolve(&spec.args, text, format!("/{name} "));
    let current = entry.meta["now"].as_str().unwrap_or_default();
    let offer = Offer {
        catalogs,
        current,
        guarded: spec.person_only(
            prefix
                .strip_prefix(&format!("/{name} "))
                .unwrap_or_default(),
        ),
    };
    let rows = option_rows(&spec.args, text, &format!("/{name} "), &offer)
        .unwrap_or_else(|| offered(args, &prefix, query, &offer));
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
            person: _,
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
fn offered(args: &ArgSpec, prefix: &str, query: &str, offer: &Offer) -> Vec<Row> {
    let current = offer.current;
    match args {
        ArgSpec::Words {
            person: _,
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
        ArgSpec::Catalog { source } => offer
            .catalogs
            .get(source)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry_row(entry, prefix, query, offer))
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
        ArgSpec::Options { args, .. } => offered(args, prefix, query, offer),
    }
}
/// A catalogue entry as a row. On a guarded level the row's label is the
/// id the line runs; the entry's own label, which whoever filled the
/// catalogue chose, is only the hint beside it.
fn entry_row(entry: &CatalogEntry, prefix: &str, query: &str, offer: &Offer) -> Option<Row> {
    if !offer.guarded {
        return choice(&entry.id, &entry.label, prefix, query, offer.current, false);
    }
    let mut row = choice(&entry.id, &entry.id, prefix, query, offer.current, false)?;
    if entry.label != entry.id {
        row.hint = entry.label.clone();
    }
    Some(row)
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

fn option_rows(args: &ArgSpec, text: &str, prefix: &str, offer: &Offer) -> Option<Vec<Row>> {
    let ArgSpec::Options { args, flags } = args else {
        return None;
    };
    let (head, query) = text.rsplit_once(' ')?;
    if !offered(args, prefix, head, offer)
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
    let (_, spec) = named(name, commands)?;
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
    fn offer<'a>(catalogs: &'a BTreeMap<String, Vec<CatalogEntry>>, current: &'a str) -> Offer<'a> {
        Offer {
            catalogs,
            current,
            guarded: false,
        }
    }
    #[test]
    fn words_options_catalog_free_and_none_have_one_argument_path() {
        let catalogs = BTreeMap::new();
        for args in [
            ArgSpec::None,
            ArgSpec::Free {
                hint: "title".into(),
            },
            ArgSpec::Words {
                person: vec![],
                values: vec!["low".into()],
                alone: vec![],
                then: None,
            },
            ArgSpec::Options {
                args: Box::new(ArgSpec::Words {
                    person: vec![],
                    values: vec!["low".into()],
                    alone: vec![],
                    then: None,
                }),
                flags: vec!["--keep".into()],
            },
        ] {
            assert_eq!(
                offered(&args, "/think ", "", &offer(&catalogs, "low")).len(),
                1
            );
        }
        assert!(
            offered(
                &ArgSpec::Catalog {
                    source: "models".into()
                },
                "/model ",
                "",
                &offer(&catalogs, "")
            )
            .is_empty()
        );
        let args = ArgSpec::Words {
            person: vec![],
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
            offered(&args, "/plugins ", "reload", &offer(&catalogs, ""))[0].target,
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
