use crate::{Shared, rows::Target};
use bingo_sdk::{component::PaletteProps, *};
use serde_json::{Value, json};
pub struct Open(pub &'static str);
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: self.0.into(),
            aliases: vec![],
            hint: if self.0 == "help" {
                "keys, in this list"
            } else {
                "reopen a session"
            }
            .into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            keys: if self.0 == "resume" {
                vec!["ctrl+r".into()]
            } else {
                vec![]
            },
        }
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let value = if self.0 == "resume" && !args.trim().is_empty() {
            json!({"resume":args.trim()})
        } else {
            json!({"open":if self.0=="help"{"keys"}else{"sessions"}})
        };
        Ok(CommandOutcome::Surface {
            fallback: Some(format!("/{} is terminal only.", self.0)),
            action: Action {
                name: "Palette".into(),
                args: value,
            },
        })
    }
}
pub struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("palette")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        let request = args["handler"].as_str().unwrap_or_default();
        let key = args["value"]["key"].as_str().unwrap_or_default();
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let props: PaletteProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let outcome = self.press(request, key, &props)?;
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        Ok(outcome)
    }
}
impl Handler {
    fn press(
        &self,
        request: &str,
        key: &str,
        props: &PaletteProps,
    ) -> Result<CommandOutcome, KernelError> {
        let mut shared = self.0.lock().map_err(crate::error)?;
        let (groups, initial) = crate::draw::groups(props, shared.recent.names());
        let rows = groups
            .iter()
            .flat_map(|group| &group.rows)
            .collect::<Vec<_>>();
        let cursor = shared
            .cursors
            .entry((props.session.clone(), request.into()))
            .or_insert_with(|| crate::Cursor {
                query: props.query.clone(),
                selected: initial,
            });
        if cursor.query != props.query {
            cursor.query = props.query.clone();
            cursor.selected = initial;
        }
        if rows
            .get(cursor.selected)
            .is_some_and(|row| matches!(row.target, Target::Inert))
        {
            cursor.selected = rows
                .iter()
                .position(|row| !matches!(row.target, Target::Inert))
                .unwrap_or(0);
        }
        let target = rows.get(cursor.selected).map(|row| row.target.clone());
        let (handled, operation) = operation(cursor, &rows, target, key, props);
        if !handled {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let Some(operation) = operation else {
            return Ok(CommandOutcome::Handled { handled: true });
        };
        if let Some(line) = operation["run"].as_str() {
            shared.recent.remember(line).map_err(crate::error)?;
        }
        if operation.get("close").is_some()
            || operation.get("run").is_some()
            || operation.get("resume").is_some()
            || operation.get("view").is_some()
            || operation.get("key").is_some()
        {
            shared
                .cursors
                .remove(&(props.session.clone(), request.into()));
        }
        surface(props, operation)
    }
}
fn surface(props: &PaletteProps, operation: Value) -> Result<CommandOutcome, KernelError> {
    let mut action = props.action.clone();
    if !action.args.is_object() {
        return Ok(CommandOutcome::Handled { handled: false });
    }
    action.args["value"] = operation;
    Ok(CommandOutcome::Surface {
        action,
        fallback: None,
    })
}

fn activate(target: Target, props: &PaletteProps) -> Value {
    match target {
        Target::Command(name) => {
            let spec = props
                .commands
                .iter()
                .find(|entry| entry.id == name)
                .and_then(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok());
            let exact = props.query.trim().strip_prefix('/').is_some_and(|query| {
                query == name
                    || spec
                        .as_ref()
                        .is_some_and(|spec| spec.aliases.iter().any(|alias| alias == query))
            });
            if !exact && spec.is_some_and(|spec| !matches!(spec.args, ArgSpec::None)) {
                json!({"edit":format!("/{name} ")})
            } else {
                json!({"run":format!("/{name}")})
            }
        }
        Target::Key(line) if line.starts_with("@key:") => {
            json!({"key":line.trim_start_matches("@key:")})
        }
        Target::Key(line) | Target::Run(line) => json!({"run":line}),
        Target::Edit(line) => json!({"edit":line}),
        Target::Session(id) | Target::View(id) if id == props.session.as_str() => {
            json!({"message":"this session is already in view"})
        }
        Target::Session(id) => json!({"resume":id}),
        Target::View(id) => json!({"view":id}),
        Target::Inert => Value::Null,
    }
}

/// A row runs a line only the person may run only when the row shows what
/// the line runs: its last argument, on the label or beside it.
fn shown(row: Option<&crate::rows::Row>, operation: Value, props: &PaletteProps) -> Value {
    let Some(line) = operation["run"].as_str() else {
        return operation;
    };
    let last = line.split_whitespace().last().unwrap_or_default();
    let visible = row.is_some_and(|row| {
        [&row.label, &row.hint]
            .iter()
            .any(|text| text.split_whitespace().any(|word| word == last))
    });
    if visible || !crate::args::person_only(line, &props.commands) {
        return operation;
    }
    json!({"message":format!("not run: this row does not show what {line} runs; type it yourself")})
}

fn operation(
    cursor: &mut crate::Cursor,
    rows: &[&crate::rows::Row],
    target: Option<Target>,
    key: &str,
    props: &PaletteProps,
) -> (bool, Option<Value>) {
    let operation = match key {
        "up" => {
            if let Some(at) = (0..cursor.selected)
                .rev()
                .find(|at| !matches!(rows[*at].target, Target::Inert))
            {
                cursor.selected = at;
            }
            None
        }
        "down" => {
            if let Some(at) = (cursor.selected + 1..rows.len())
                .find(|at| !matches!(rows[*at].target, Target::Inert))
            {
                cursor.selected = at;
            }
            None
        }
        "ctrl+c" => Some(json!({"close":true})),
        "esc" => Some(
            crate::args::parent(&props.query, &props.commands)
                .map(|edit| json!({"edit":edit}))
                .unwrap_or_else(|| json!({"close":true})),
        ),
        "backspace" if props.query.is_empty() || props.query == "/" => Some(json!({"close":true})),
        "tab" => target.and_then(|target| match target {
            Target::Command(name) => Some(json!({"edit":format!("/{name} ")})),
            _ => None,
        }),
        "enter" => target
            .filter(|target| !matches!(target, Target::Inert))
            .map(|target| activate(target, props))
            .map(|operation| shown(rows.get(cursor.selected).copied(), operation, props))
            .or_else(|| {
                (props.query.starts_with('/') && props.query.len() > 1)
                    .then(|| json!({"run":props.query.trim()}))
            }),
        _ => return (false, None),
    };
    (true, operation)
}

#[cfg(test)]
mod review_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    fn props() -> PaletteProps {
        serde_json::from_value(json!({"query":"", "open":"keys", "commands":[], "keyHints":{}, "bindings":{}, "session":"s", "conversation":[], "sessions":[], "action":{"name":"client", "args":{}}, "catalogs":{}})).unwrap()
    }
    #[test]
    fn exact_commands_and_aliases_run_while_partial_names_complete() {
        let mut props = props();
        props.commands = vec![CatalogEntry {
            id: "plugins".into(),
            label: "plugins".into(),
            meta: json!({"name":"plugins","hint":"plugins","args":{"kind":"words","values":["enable"],"alone":[]},"instant":true,"family":"session","aliases":["modules"]}),
        }];
        for query in ["/plugins", "/modules"] {
            props.query = query.into();
            assert_eq!(
                activate(Target::Command("plugins".into()), &props),
                json!({"run":"/plugins"})
            );
        }
        props.query = "/plug".into();
        assert_eq!(
            activate(Target::Command("plugins".into()), &props),
            json!({"edit":"/plugins "})
        );
    }
    #[test]
    fn informational_and_interrupt_keys_are_not_actionable() {
        let mut props = props();
        for key in [
            "ctrl+a/e",
            "esc esc",
            "up/down",
            "alt+b/f",
            "1-9 · y/a/n",
            "v · drag",
            "click",
            "esc",
            "ctrl+c",
            "ctrl+d",
        ] {
            props.key_hints.insert(key.into(), "informational".into());
        }
        let (groups, _) = crate::draw::groups(&props, &[]);
        assert!(
            groups
                .iter()
                .flat_map(|g| &g.rows)
                .all(|row| !matches!(row.target, Target::Key(_)))
        );
    }
    #[test]
    fn ctrl_c_clears_the_closed_openings_cursor() {
        let home = tempfile::tempdir().unwrap();
        let props = props();
        let state = Arc::new(Mutex::new(crate::State {
            recent: crate::recent::Recent::load(home.path().join("recent.json")),
            cursors: Default::default(),
        }));
        state.lock().unwrap().cursors.insert(
            (props.session.clone(), "opening".into()),
            Default::default(),
        );
        let outcome = Handler(state.clone())
            .press("opening", "ctrl+c", &props)
            .unwrap();
        assert!(
            matches!(outcome, CommandOutcome::Surface { action, .. } if action.args["value"]["close"] == true)
        );
        assert!(state.lock().unwrap().cursors.is_empty());
    }
    #[test]
    fn arrow_navigation_skips_inert_rows_and_enter_releases_its_cursor() {
        let home = tempfile::tempdir().unwrap();
        let mut props = props();
        for key in ["shift+tab", "esc", "ctrl+o", "ctrl+c"] {
            props.key_hints.insert(key.into(), key.into());
        }
        let state = Arc::new(Mutex::new(crate::State {
            recent: crate::recent::Recent::load(home.path().join("recent.json")),
            cursors: Default::default(),
        }));
        let handler = Handler(state.clone());
        handler.press("opening", "down", &props).unwrap();
        let outcome = handler.press("opening", "enter", &props).unwrap();
        assert!(
            matches!(outcome, CommandOutcome::Surface { action, .. } if action.args["value"]["key"] == "ctrl+o")
        );
        assert!(state.lock().unwrap().cursors.is_empty());
    }
    /// A link that fills the plugin catalogue names the id `evil` "good".
    /// The trust row shows `evil`, and a row that hides it runs nothing.
    #[test]
    fn a_person_only_row_runs_only_the_argument_it_shows() {
        let home = tempfile::tempdir().unwrap();
        let mut props = props();
        props.open = bingo_sdk::component::PaletteOpen::Commands;
        props.commands = vec![CatalogEntry {
            id: "plugins".into(),
            label: "plugins".into(),
            meta: json!({"name":"plugins","hint":"plugins","family":"session","args":{"kind":"words","values":["enable","trust"],"person":["trust"],"alone":[],"then":{"kind":"catalog","source":"plugins"}}}),
        }];
        props.catalogs.insert(
            "plugins".into(),
            vec![CatalogEntry {
                id: "evil".into(),
                label: "good".into(),
                meta: Value::Null,
            }],
        );
        props.query = "/plugins trust ".into();
        let state = Arc::new(Mutex::new(crate::State {
            recent: crate::recent::Recent::load(home.path().join("recent.json")),
            cursors: Default::default(),
        }));
        let (groups, _) = crate::draw::groups(&props, &[]);
        let row = &groups[0].rows[0];
        assert_eq!((row.label.as_str(), row.hint.as_str()), ("evil", "good"));
        let outcome = Handler(state).press("opening", "enter", &props).unwrap();
        assert!(
            matches!(&outcome, CommandOutcome::Surface { action, .. } if action.args["value"]["run"] == "/plugins trust evil"),
            "{outcome:?}"
        );
        let forged = crate::rows::Row {
            label: "good".into(),
            hint: String::new(),
            alert: None,
            now: String::new(),
            marks: vec![],
            hint_marks: vec![],
            target: Target::Run("/plugins trust evil".into()),
        };
        let run = json!({"run":"/plugins trust evil"});
        assert!(shown(Some(&forged), run.clone(), &props)["message"].is_string());
        props.commands[0].meta["args"]["person"] = json!([]);
        assert_eq!(shown(Some(&forged), run.clone(), &props), run);
    }
}
