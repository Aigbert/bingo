//! Memory file calls place their own transcript rows.

use std::path::{Path, PathBuf};

use bingo_sdk::SessionState;
use serde_json::Value;

use super::DIRECTORIES as KIND;
/// The namespace and the kind the directories are published under.
use crate::baseline::PLUGIN;
/// The two scopes, each a directory.
const SCOPES: [&str; 2] = ["user", "project"];
/// The index every scope keeps, which draws under its own name: it is not a
/// fact, and a person knows it by this one.
const INDEX: &str = "MEMORY.md";
/// The argument of the three file tools that names the file.
const FILE_PATH: &str = "file_path";
/// The three tools that touch a file, and what each is when the file is a
/// memory. Any other tool on the same path — a `Grep`, a `Glob` — is what it
/// was: it is searching, not remembering.
const VERBS: [(&str, &str); 3] = [
    ("Read", "Recall from memory"),
    ("Write", "Write memory"),
    ("Edit", "Edit memory"),
];

/// Where the session's memories are, as far as the journal says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Directories(Vec<PathBuf>);

pub fn of(state: &SessionState) -> Directories {
    let Some(published) = state
        .extensions
        .get(PLUGIN)
        .and_then(|kinds| kinds.get(KIND))
    else {
        return Directories::default();
    };
    Directories(
        SCOPES
            .iter()
            .filter_map(|scope| published.get(scope).and_then(Value::as_str))
            .map(PathBuf::from)
            .collect(),
    )
}

/// The row's name and argument when the call is on a memory file — the verb
/// for the tool and the memory's own name — and nothing when it is not.
pub fn call(
    name: &str,
    input: &Value,
    cwd: &str,
    dirs: &Directories,
) -> Option<(&'static str, String)> {
    let (_, verb) = VERBS.iter().find(|(tool, _)| *tool == name)?;
    let path = absolute(input.get(FILE_PATH)?.as_str()?, cwd);
    let inside = dirs
        .0
        .iter()
        .any(|dir| path.parent() == Some(dir.as_path()));
    inside.then(|| (*verb, about(&path)))
}

/// A path the model wrote relative to where it was working.
fn absolute(path: &str, cwd: &str) -> PathBuf {
    let path = Path::new(path);
    match path.is_absolute() {
        true => path.to_path_buf(),
        false => Path::new(cwd).join(path),
    }
}

/// The memory's name as the index links it — the slug, without `.md` — and
/// the index by its own name.
fn about(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if name == INDEX {
        return name.into_owned();
    }
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

pub(crate) struct MemoryRows;

#[async_trait::async_trait]
impl bingo_sdk::Hook for MemoryRows {
    fn id(&self) -> &str {
        "context:memory-rows"
    }

    fn matcher(&self) -> bingo_sdk::HookMatcher {
        bingo_sdk::HookMatcher {
            points: vec![bingo_sdk::HookPoint::BeforeTool],
            tool: None,
        }
    }

    async fn before_tool(
        &self,
        call: &mut bingo_sdk::ToolCall,
        cx: &bingo_sdk::HookContext,
    ) -> bingo_sdk::HookOutcome {
        if VERBS.iter().any(|(name, _)| *name == call.name) {
            place(call, cx).await;
        }
        bingo_sdk::HookOutcome::Continue
    }
}

async fn place(call: &bingo_sdk::ToolCall, cx: &bingo_sdk::HookContext) {
    let Some(item) = &cx.item else { return };
    let Ok(opened) = cx
        .host
        .open(
            bingo_sdk::SessionSelector::ById {
                id: cx.session.clone(),
            },
            bingo_sdk::ClientIdentity {
                name: "memory rows".into(),
                surface: PLUGIN.into(),
            },
            Default::default(),
        )
        .await
    else {
        return;
    };
    let dirs = of(&opened.snapshot);
    let Some((title, badge)) =
        self::call(&call.name, &call.input, &cx.cwd.to_string_lossy(), &dirs)
    else {
        return;
    };
    let row = row(item, title, badge);
    let payload = bingo_sdk::slot::with(serde_json::json!({}), [row]);
    if let Err(error) = cx
        .host
        .extend(&cx.session, PLUGIN, &format!("row:{item}"), payload)
        .await
    {
        tracing::debug!(%error, "the memory row was not placed");
    }
}

fn row(item: &bingo_sdk::ItemId, title: &str, badge: String) -> bingo_sdk::slot::Slot {
    bingo_sdk::slot::Slot::Row {
        caption: None,
        item: item.clone(),
        mark: None,
        title: Some(title.into()),
        badge: Some(badge),
        body: None,
        actions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn the_hook_places_only_memory_calls_on_the_gated_item() {
        use bingo_sdk::{
            Event, Hook, HookContext, ToolCall,
            testing::{HostCall, TestHost},
        };
        let Event::SessionUpdated { summary } = serde_json::from_str(include_str!(
            "../../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
        ))
        .unwrap() else {
            panic!("summary")
        };
        let base = TestHost::with_sessions(vec![summary.clone()]);
        let host = TestHost::default();
        host.on_open(move |selector, who, options| {
            let base = base.handle();
            async move {
                let mut opened = base.open(selector, who, options).await?;
                opened
                    .snapshot
                    .extensions
                    .entry(PLUGIN.into())
                    .or_default()
                    .insert(KIND.into(), json!({"user":"/data/memory/user"}));
                Ok(opened)
            }
        });
        host.on_extend(|_, _, _, _| async { Ok(()) });
        let item = bingo_sdk::ItemId::from_raw("itm_memory");
        let mut cx = HookContext {
            item: Some(item.clone()),
            session: summary.id,
            turn: None,
            cwd: "/data".into(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        let mut call = ToolCall {
            call_id: "memory".into(),
            name: "Read".into(),
            input: json!({"file_path":"memory/user/the-build.md"}),
        };
        MemoryRows.before_tool(&mut call, &cx).await;
        call.input = json!({"file_path":"src/lib.rs"});
        MemoryRows.before_tool(&mut call, &cx).await;
        cx.item = None;
        call.input = json!({"file_path":"memory/user/the-build.md"});
        MemoryRows.before_tool(&mut call, &cx).await;
        let writes: Vec<_> = host
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                HostCall::Extend {
                    plugin,
                    kind,
                    payload,
                    ..
                } => Some((plugin, kind, payload)),
                _ => None,
            })
            .collect();
        assert_eq!(writes.len(), 1);
        assert_eq!((&*writes[0].0, &*writes[0].1), (PLUGIN, "row:itm_memory"));
        let slots: Vec<bingo_sdk::slot::Slot> =
            serde_json::from_value(writes[0].2["slots"].clone()).unwrap();
        assert_eq!(
            slots,
            [bingo_sdk::slot::Slot::Row {
                item,
                mark: None,
                title: Some("Recall from memory".into()),
                badge: Some("the-build".into()),
                caption: None,
                body: None,
                actions: vec![],
            }]
        );
    }

    fn published() -> Directories {
        Directories(vec![
            "/data/memory/user".into(),
            "/data/memory/web-2bf6c26a7362cd1f".into(),
        ])
    }

    #[test]
    fn a_file_tool_on_a_memory_is_the_memory_verb_and_the_slug() {
        let item = bingo_sdk::ItemId::from_raw("itm_memory");
        assert_eq!(
            row(&item, "Recall from memory", "the-build".into()),
            bingo_sdk::slot::Slot::Row {
                caption: None,
                item,
                mark: None,
                title: Some("Recall from memory".into()),
                badge: Some("the-build".into()),
                body: None,
                actions: vec![],
            }
        );
        let dirs = published();
        let read = json!({"file_path": "/data/memory/web-2bf6c26a7362cd1f/the-build.md"});
        assert_eq!(
            call("Read", &read, "/work", &dirs),
            Some(("Recall from memory", "the-build".to_string()))
        );
        let write = json!({"file_path": "/data/memory/user/MEMORY.md", "content": "- …"});
        assert_eq!(
            call("Write", &write, "/work", &dirs),
            Some(("Write memory", "MEMORY.md".to_string()))
        );
        let edit = json!({"file_path": "/data/memory/user/prefers-short-replies.md"});
        assert_eq!(
            call("Edit", &edit, "/work", &dirs),
            Some(("Edit memory", "prefers-short-replies".to_string()))
        );
    }

    #[test]
    fn a_relative_path_is_read_from_where_the_session_works() {
        let dirs = published();
        let read = json!({"file_path": "memory/user/a-habit.md"});
        assert_eq!(
            call("Read", &read, "/data", &dirs),
            Some(("Recall from memory", "a-habit".to_string()))
        );
        assert_eq!(call("Read", &read, "/elsewhere", &dirs), None);
    }

    #[test]
    fn anything_else_is_what_it_was() {
        let dirs = published();
        let elsewhere = json!({"file_path": "/work/src/lib.rs"});
        assert_eq!(call("Read", &elsewhere, "/work", &dirs), None);
        let search = json!({"pattern": "habit", "path": "/data/memory/user"});
        assert_eq!(call("Grep", &search, "/work", &dirs), None);
        let deeper = json!({"file_path": "/data/memory/user/notes/a.md"});
        assert_eq!(
            call("Read", &deeper, "/work", &dirs),
            None,
            "a memory is one file deep"
        );
        let read = json!({"file_path": "/data/memory/user/a-habit.md"});
        assert_eq!(call("Read", &read, "/work", &Directories::default()), None);
    }
}
