use crate::rows::{Row, Target, marks};
use bingo_sdk::{ArgSpec, AuthStatus, CatalogEntry, CommandSpec};
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
    /// The first level's words the session offers, when fewer than the
    /// spec's: a model's own thinking levels.
    only: Option<Vec<String>>,
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
        only: (prefix == format!("/{name} "))
            .then(|| serde_json::from_value(entry.meta["offered"].clone()).ok())
            .flatten(),
    };
    let rows = option_rows(&spec.args, text, &format!("/{name} "), &offer)
        .unwrap_or_else(|| offered(args, &prefix, query, &offer));
    let (rows, selected) = if spec.opens {
        choices_first(rows, &prefix, query)
    } else {
        let selected = rows.iter().position(|row| row.now == "now").unwrap_or(0);
        (rows, selected)
    };
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
            labels: _,
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
            labels,
            person: _,
            values,
            alone,
            then,
        } => values
            .iter()
            .filter(|value| offer.only.as_ref().is_none_or(|only| only.contains(value)))
            .filter_map(|value| {
                word_row(
                    value,
                    labels.get(value),
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
                format!("⏎ {}", preview(prefix, query))
            },
            hint: String::new(),
            now: if current.is_empty() {
                String::new()
            } else {
                format!("now {current}")
            },
            alert: None,
            marks: vec![],
            target: Target::Run(format!("{prefix}{query}").trim_end().into()),
        }],
        ArgSpec::None => vec![Row {
            label: format!("runs {}", format!("{prefix}{query}").trim_end()),
            hint: String::new(),
            now: String::new(),
            alert: None,
            marks: vec![],
            target: Target::Run(format!("{prefix}{query}").trim_end().into()),
        }],
        ArgSpec::Options { args, .. } => offered(args, prefix, query, offer),
    }
}
/// A word as a row. A word drawn by a label carries its value as the hint,
/// so the row still shows what the line runs.
fn word_row(
    value: &str,
    label: Option<&String>,
    prefix: &str,
    query: &str,
    current: &str,
    more: bool,
) -> Option<Row> {
    let label = label.map_or(value, String::as_str);
    let mut row = choice(value, label, prefix, query, current, more)?;
    if label != value {
        row.hint = value.into();
    }
    Some(row)
}
/// A catalogue entry as a row. On a guarded level the row's label is the
/// id the line runs; the entry's own label, which whoever filled the
/// catalogue chose, is only the hint beside it.
fn entry_row(entry: &CatalogEntry, prefix: &str, query: &str, offer: &Offer) -> Option<Row> {
    let mut row = if offer.guarded {
        let mut row = choice(&entry.id, &entry.id, prefix, query, offer.current, false)?;
        if entry.label != entry.id {
            row.hint = entry.label.clone();
        }
        row
    } else {
        choice(&entry.id, &entry.label, prefix, query, offer.current, false)?
    };
    if row.now.is_empty()
        && let Some(signed) = signed(entry)
    {
        row.now = signed.into();
    }
    Some(row)
}
/// Whether a provider holds a key, from the `auth` its catalogue entry
/// carries; nothing for an entry that is not a provider or needs none.
fn signed(entry: &CatalogEntry) -> Option<&'static str> {
    match serde_json::from_value(entry.meta["auth"].clone()).ok()? {
        AuthStatus::Ready { .. } => Some("● signed in"),
        AuthStatus::Missing { .. } => Some("○ signed out"),
        AuthStatus::Expired { .. } => Some("○ expired"),
        AuthStatus::NotApplicable => None,
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
    let marks = marks(label, query).or_else(|| marks(value, query))?;
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

/// A command that opens its own chooser lists its choices with the one in
/// use selected; a filter selects the best match, and the line as typed —
/// which opens the chooser on that filter — waits one row past them.
fn choices_first(mut rows: Vec<Row>, prefix: &str, query: &str) -> (Vec<Row>, usize) {
    let line = format!("{prefix}{query}").trim_end().to_string();
    let target = Target::Run(line.clone());
    if let Some(at) = rows.iter().position(|row| row.target == target) {
        return (rows, at);
    }
    if query.is_empty() {
        let at = rows.iter().position(|row| row.now == "now").unwrap_or(0);
        return (rows, at);
    }
    rows.push(Row {
        label: format!("runs {line}"),
        hint: String::new(),
        now: String::new(),
        alert: None,
        marks: vec![],
        target,
    });
    (rows, 0)
}

fn preview(prefix: &str, query: &str) -> String {
    match prefix {
        "/rename " => format!("names this session “{query}”"),
        "/compact " => format!("compacts, keeping {query}"),
        _ => format!("runs {prefix}{query}"),
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
    use serde_json::json;
    fn offer<'a>(catalogs: &'a BTreeMap<String, Vec<CatalogEntry>>, current: &'a str) -> Offer<'a> {
        Offer {
            catalogs,
            current,
            guarded: false,
            only: None,
        }
    }
    #[test]
    fn a_provider_row_says_whether_it_holds_a_key() {
        let catalogs = BTreeMap::from([(
            "providers".into(),
            [
                ("anthropic", json!({"kind": "ready"})),
                ("codex", json!({"kind": "missing", "hint": "sign in"})),
                ("agent", json!({"kind": "notApplicable"})),
            ]
            .map(|(id, auth)| CatalogEntry {
                id: id.into(),
                label: id.into(),
                meta: json!({ "auth": auth }),
            })
            .to_vec(),
        )]);
        let commands = vec![CatalogEntry {
            id: "login".into(),
            label: "login".into(),
            meta: json!({"name":"login","hint":"sign in to a provider","family":"session","opens":true,"args":{"kind":"catalog","source":"providers"}}),
        }];
        let login = arguments("/login ", &commands, &catalogs).unwrap();
        let rows = login
            .rows
            .iter()
            .map(|row| (row.label.as_str(), row.now.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            [
                ("anthropic", "● signed in"),
                ("codex", "○ signed out"),
                ("agent", "")
            ]
        );
    }
    #[test]
    fn a_free_argument_shows_its_hint_then_what_enter_does() {
        let commands = vec![CatalogEntry {
            id: "rename".into(),
            label: "rename".into(),
            meta: json!({"name":"rename","hint":"name this session","family":"session","now":"label the total","args":{"kind":"free","hint":"a name for this session"}}),
        }];
        let bare = arguments("/rename ", &commands, &BTreeMap::new()).unwrap();
        assert_eq!(bare.rows[0].label, "a name for this session");
        assert_eq!(bare.rows[0].now, "now label the total");
        let typed = arguments("/rename totals", &commands, &BTreeMap::new()).unwrap();
        assert_eq!(typed.rows[0].label, "⏎ names this session “totals”");
    }
    #[test]
    fn a_session_offering_fewer_words_lists_only_those() {
        let commands = vec![CatalogEntry {
            id: "think".into(),
            label: "think".into(),
            meta: serde_json::json!({"name":"think","hint":"how hard it thinks","family":"session","now":"low","offered":["low","high","off"],"args":{"kind":"options","args":{"kind":"words","values":["minimal","low","medium","high","off"]},"flags":["--keep"]}}),
        }];
        let offered = arguments("/think ", &commands, &BTreeMap::new()).unwrap();
        let labels = offered
            .rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, ["low", "high", "off"]);
        assert_eq!(offered.rows[offered.selected].label, "low");
        let flags = arguments("/think high ", &commands, &BTreeMap::new()).unwrap();
        assert_eq!(
            flags.rows[0].label, "--keep",
            "a later level is not narrowed"
        );
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
                labels: Default::default(),
                person: vec![],
                values: vec!["low".into()],
                alone: vec![],
                then: None,
            },
            ArgSpec::Options {
                args: Box::new(ArgSpec::Words {
                    labels: Default::default(),
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
            labels: Default::default(),
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
    #[test]
    fn a_command_that_opens_takes_the_best_match_and_keeps_the_filter() {
        let catalogs = BTreeMap::from([(
            "models".into(),
            vec![CatalogEntry {
                id: "work/large".into(),
                label: "large".into(),
                meta: serde_json::Value::Null,
            }],
        )]);
        let commands = vec![CatalogEntry {
            id: "model".into(),
            label: "model".into(),
            meta: serde_json::json!({"name":"model","hint":"choose","family":"session","opens":true,"now":"work/large","args":{"kind":"catalog","source":"models"}}),
        }];
        // Enter takes the best match; the filter as typed, which opens the
        // picker on that provider, waits one row past it.
        let filter = arguments("/model work/", &commands, &catalogs).unwrap();
        assert_eq!(
            filter.rows[filter.selected].target,
            Target::Run("/model work/large".into())
        );
        assert_eq!(filter.rows[1].target, Target::Run("/model work/".into()));
        let unmatched = arguments("/model zz", &commands, &catalogs).unwrap();
        assert_eq!(
            unmatched.rows[unmatched.selected].target,
            Target::Run("/model zz".into())
        );
        let exact = arguments("/model work/large", &commands, &catalogs).unwrap();
        assert_eq!(exact.rows.len(), 1);
        assert_eq!(
            exact.rows[exact.selected].target,
            Target::Run("/model work/large".into())
        );
        let bare = arguments("/model ", &commands, &catalogs).unwrap();
        assert_eq!(bare.rows.len(), 1, "no row stands for the picker");
        assert_eq!(bare.rows[bare.selected].now, "now");
    }
    /// A word's label is what the row says; the value is what it runs and
    /// what the command's `now` names, so every mode is found, not only the
    /// one whose label happens to be its value.
    #[test]
    fn a_labelled_word_is_drawn_by_label_and_marked_by_value() {
        let modes = [
            ("default", "default"),
            ("acceptEdits", "accept edits"),
            ("plan", "plan"),
            ("bypassPermissions", "bypass"),
            ("dontAsk", "don't ask"),
        ];
        for (value, label) in modes {
            let commands = vec![CatalogEntry {
                id: "permission".into(),
                label: "permission".into(),
                meta: serde_json::json!({"name":"permission","hint":"mode","family":"session","now":value,"args":{
                    "kind":"words","values":modes.map(|(v, _)| v),
                    "person":modes.map(|(v, _)| v),
                    "alone":modes.map(|(v, _)| v),
                    "labels":{"acceptEdits":"accept edits","bypassPermissions":"bypass","dontAsk":"don't ask"}}}),
            }];
            let offered = arguments("/permission ", &commands, &BTreeMap::new()).unwrap();
            let chosen = &offered.rows[offered.selected];
            assert_eq!(chosen.now, "now", "{value}");
            assert_eq!(chosen.label, label, "{value}");
            assert_eq!(
                chosen.hint,
                if label == value { "" } else { value },
                "{value}"
            );
            assert_eq!(chosen.target, Target::Run(format!("/permission {value}")));
            assert_eq!(
                offered.rows.iter().filter(|row| row.now == "now").count(),
                1,
                "{value}"
            );
            let typed = arguments("/permission accept", &commands, &BTreeMap::new()).unwrap();
            assert_eq!(typed.rows[0].label, "accept edits");
        }
    }
}
