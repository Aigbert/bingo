//! Registered names resolve their callback after the owner's reload check.
use super::params::{RegisterCommand, RegisterTool};
use bingo_plugin_host::{
    callback::{self, Registry},
    ops::Caller,
    tool_name::tool_name,
};
use bingo_sdk::*;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
enum Spec {
    Tool(ToolSpec),
    Command(CommandSpec),
}
#[derive(Clone)]
struct Named<T> {
    plugin: String,
    callback: String,
    spec: T,
    stopped: CancellationToken,
    registry: Arc<Registry>,
}
#[derive(Default)]
pub struct Registrations(Mutex<BTreeMap<(String, String), Named<Spec>>>);
impl Registrations {
    pub fn tool(&self, caller: &Caller, input: RegisterTool) -> Result<String, String> {
        let name = tool_name(&caller.plugin, &input.spec.name);
        let mut spec = input.spec;
        spec.name = name.clone();
        self.insert(caller, input.callback, Spec::Tool(spec))?;
        Ok(name)
    }
    pub fn command(&self, caller: &Caller, input: RegisterCommand) -> Result<String, String> {
        if let Some(taken) = input.spec.foreign_handler(&caller.plugin) {
            return Err(format!(
                "{taken} answers another plugin's controls; a mod may name only its own {}",
                CommandSpec::ui_name(&caller.plugin)
            ));
        }
        let name = input.spec.name.clone();
        self.insert(caller, input.callback, Spec::Command(input.spec))?;
        Ok(name)
    }
    fn insert(&self, caller: &Caller, callback: String, spec: Spec) -> Result<(), String> {
        let registry = caller
            .host
            .service(&callback::KEY)
            .ok_or("callback registry is unavailable")?;
        let stopped = registry.get(&caller.plugin)?.stopped();
        let named = Named {
            plugin: caller.plugin.clone(),
            callback: callback.clone(),
            spec,
            stopped,
            registry,
        };
        let mut entries = self.0.lock().unwrap_or_else(|error| error.into_inner());
        entries.retain(|_, entry| !entry.stopped.is_cancelled());
        entries.insert((caller.plugin.clone(), callback), named);
        Ok(())
    }
    pub fn remove(&self, plugin: &str) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(owner, _), _| owner != plugin);
    }
    pub fn has_tools(&self, plugin: &str) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .any(|entry| {
                entry.plugin == plugin
                    && matches!(entry.spec, Spec::Tool(_))
                    && !entry.stopped.is_cancelled()
            })
    }
    pub fn contents(&self, plugin: &str) -> plugin::PluginContents {
        let entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let mut contents = plugin::PluginContents::default();
        for entry in entries
            .values()
            .filter(|entry| entry.plugin == plugin && !entry.stopped.is_cancelled())
        {
            match &entry.spec {
                Spec::Command(spec) if !spec.hidden => contents.commands.push(spec.clone()),
                Spec::Tool(spec) => contents.tools.push(spec.name.clone()),
                _ => {}
            }
        }
        contents
    }
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .filter_map(|entry| match &entry.spec {
                Spec::Tool(spec) if !entry.stopped.is_cancelled() => {
                    Some(Arc::new(RegisteredTool(entry.with_spec(spec.clone()))) as Arc<dyn Tool>)
                }
                _ => None,
            })
            .collect()
    }
    pub fn commands(&self) -> Vec<Arc<dyn Command>> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .values()
            .filter_map(|entry| match &entry.spec {
                Spec::Command(spec) if !entry.stopped.is_cancelled() => {
                    Some(Arc::new(RegisteredCommand(entry.with_spec(spec.clone())))
                        as Arc<dyn Command>)
                }
                _ => None,
            })
            .collect()
    }
}
impl<T> Named<T> {
    fn with_spec<S>(&self, spec: S) -> Named<S> {
        Named {
            plugin: self.plugin.clone(),
            callback: self.callback.clone(),
            spec,
            stopped: self.stopped.clone(),
            registry: self.registry.clone(),
        }
    }
    async fn run(&self, input: Value, cx: &HookContext) -> Result<Value, String> {
        let executor = self.registry.prepared(&self.plugin).await?;
        executor.call(&self.callback, input, cx).await
    }
}
struct RegisteredTool(Named<ToolSpec>);
#[async_trait::async_trait]
impl Tool for RegisteredTool {
    fn spec(&self) -> ToolSpec {
        self.0.spec.clone()
    }
    async fn call(&self, input: Value, cx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let context = HookContext {
            item: Some(cx.item.clone()),
            session: cx.session.clone(),
            turn: Some(cx.turn.clone()),
            cwd: cx.cwd.clone(),
            provider: None,
            model: None,
            host: cx.host.clone(),
        };
        let answer = self
            .0
            .run(input, &context)
            .await
            .map_err(ToolError::Failed)?;
        serde_json::from_value(answer).map_err(|error| ToolError::Failed(error.to_string()))
    }
}
struct RegisteredCommand(Named<CommandSpec>);
#[async_trait::async_trait]
impl Command for RegisteredCommand {
    fn spec(&self) -> CommandSpec {
        self.0.spec.clone()
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let context = HookContext {
            item: None,
            session: cx.session.clone(),
            turn: None,
            cwd: cx.cwd.clone(),
            provider: None,
            model: None,
            host: cx.host.clone(),
        };
        let result = tokio::select! {
            biased;
            _ = cx.cancel.cancelled() => return Err(failed("command cancelled")),
            result = self.0.run(Value::String(args.into()), &context) => result.map_err(failed)?,
        };
        serde_json::from_value(result).map_err(failed)
    }
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_plugin_host::state::Store;

    fn caller(root: &std::path::Path) -> Caller {
        Caller {
            plugin: "aaa".into(),
            host: testing::TestHost::default().handle(),
            state: Store::new(root, "aaa"),
        }
    }
    fn named(name: &str, aliases: &[&str]) -> RegisterCommand {
        serde_json::from_value(serde_json::json!({
            "session": "s",
            "spec": {"name": name, "aliases": aliases, "hint": "", "args": {"kind": "none"},
                "family": "fixture"},
            "callback": "command/1",
        }))
        .unwrap()
    }

    #[test]
    fn a_mod_cannot_register_another_plugins_handler() {
        let root = tempfile::tempdir().unwrap();
        let registrations = Registrations::default();
        for (name, aliases) in [("counter:ui", &[][..]), ("hello", &["counter:ui"][..])] {
            let refused = registrations
                .command(&caller(root.path()), named(name, aliases))
                .unwrap_err();
            assert!(refused.contains("counter:ui"), "{refused}");
        }
        for own in ["aaa:ui", "hello"] {
            let refused = registrations
                .command(&caller(root.path()), named(own, &[]))
                .unwrap_err();
            assert!(!refused.contains("controls"), "{own}: {refused}");
        }
        assert!(registrations.commands().is_empty());
    }
}
