// `bingo plugins list | enable | disable <name>`: the headless twin of
// `/plugins` (ADR-0057 §6).
//
// It runs before a kernel exists, as `bingo mcp` does: what `enable` and
// `disable` write is what the *next* run reads, and `list` says what the next
// start will do — the same verdicts on the same composition, plus the plugin
// directories this machine holds, read the way a run reads them and spawned
// the way a listing spawns them, which is not at all.
//
// Stdout carries the answer and nothing else; a directory that would not read
// is a diagnostic on stderr, as every diagnostic is.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bingo_core::plugins::{ignored, needed, state};
use bingo_core::settings::{self, Claim, Layer};
use bingo_plugin_rpc::discovery::{self, Found};
use bingo_plugin_rpc::notice::Notices;
use bingo_sdk::{Env, ErrorCode, KernelError, Plugin, PluginStatus, SWITCHED_OFF};
use bingo_surface_print::notice_report;
use clap::Subcommand;

/// What one `bingo plugins` line asks for, in clap's own shape.
#[derive(Clone, Debug, Subcommand)]
pub enum Action {
    /// Parse a plugin and report its protocol-2 contributions.
    Validate { dir: std::path::PathBuf },
    /// Run isolated fake-model scenarios against a plugin.
    Test { dir: std::path::PathBuf },
    /// Every plugin the next start will consider, and whether it will run.
    List,
    /// Turn one on in the user settings layer.
    Enable {
        /// The plugin's name, as `list` writes it.
        name: String,
    },
    /// Turn one off in the user settings layer.
    Disable {
        /// The plugin's name, as `list` writes it.
        name: String,
    },
}

/// Do it, and answer with the one thing stdout carries.
pub fn run(
    action: &Action,
    env: &Env,
    cwd: &Path,
    layers: &[Layer],
    composed: Vec<Box<dyn Plugin>>,
) -> Result<i32, KernelError> {
    let settings = configuration(&composed, layers)?;
    let switched_off = settings.kernel.switched_off();
    for (_, text) in &settings.notices {
        eprintln!("{text}");
    }
    let runtime: bingo_plugin_rpc::Settings = serde_json::from_value(
        settings
            .plugins
            .get(bingo_plugin_rpc::PluginRpcPlugin::default().manifest().id)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .map_err(|error| invalid(error.to_string()))?;
    let mut listed = listing(&composed, env, cwd, &switched_off, &runtime.plugin_dirs);
    for status in &mut listed {
        if let Some(execution) = &mut status.execution
            && let Some(trust) = settings.kernel.plugin_trust.get(&status.id)
        {
            execution.grant = bingo_plugin_host::grant::grant(
                execution.runtime,
                execution.origin,
                Some(trust),
                &execution.grant.uses.iter().cloned().collect::<Vec<_>>(),
            );
        }
    }
    let said = match action {
        Action::Validate { .. } | Action::Test { .. } => {
            return Err(invalid(
                "author kit action reached the switch command".into(),
            ));
        }
        Action::List => list_with_uses(&listed, &composed, &settings.kernel.plugin_trust),
        Action::Enable { name } => switched(env, &composed, &listed, name, true)?,
        Action::Disable { name } => switched(env, &composed, &listed, name, false)?,
    };
    println!("{said}");
    Ok(0)
}

/// The names the layers turned off. The claims come from the composition, so
/// the merge is the one a run would do and a wrong type is refused here too.
fn configuration(
    composed: &[Box<dyn Plugin>],
    layers: &[Layer],
) -> Result<settings::Merged, KernelError> {
    let claims: Vec<Claim> = composed
        .iter()
        .filter_map(|plugin| Claim::from_manifest(plugin.manifest()))
        .collect();
    let merged = settings::merge(layers, &claims).map_err(|e| invalid(e.to_string()))?;
    Ok(merged)
}

/// What the next start will do: the build's own plugins first, then what this
/// machine has installed under `plugins/`.
fn listing(
    composed: &[Box<dyn Plugin>],
    env: &Env,
    cwd: &Path,
    switched_off: &BTreeSet<String>,
    explicit: &[PathBuf],
) -> Vec<PluginStatus> {
    let mut listed = built_in(composed, switched_off);
    listed.extend(external(env, cwd, switched_off, explicit));
    listed
}

/// This build's own, under the same verdict the registry would reach: the
/// switches first, then the requirements, to a fixpoint (ADR-0057 §2).
fn built_in(composed: &[Box<dyn Plugin>], switched_off: &BTreeSet<String>) -> Vec<PluginStatus> {
    composed
        .iter()
        .zip(bingo_core::host::standing(composed, switched_off))
        .map(|(plugin, reason)| match reason {
            Some(reason) => PluginStatus::disabled(plugin.manifest(), reason),
            None => PluginStatus::loaded(plugin.manifest()),
        })
        .collect()
}

/// The plugin directories this machine holds. Nothing is spawned, so nothing
/// here knows whether a process would answer: what it says is what the switch
/// says, which is what the next start will act on.
fn external(
    env: &Env,
    cwd: &Path,
    switched_off: &BTreeSet<String>,
    explicit: &[PathBuf],
) -> Vec<PluginStatus> {
    let notices = Notices::default();
    let found = discovery::with_explicit(&discovery::dirs(env, cwd), explicit, &notices);
    let human = std::io::IsTerminal::is_terminal(&std::io::stderr());
    for notice in notices.drain() {
        eprintln!("{}", notice_report(&notice.code, &notice.text, human));
    }
    found
        .into_iter()
        .map(|found| installed(found, switched_off))
        .collect()
}

fn installed(found: Found, switched_off: &BTreeSet<String>) -> PluginStatus {
    let runtime = match found.manifest.launch {
        bingo_plugin_host::manifest::Launch::Entry(_) => bingo_sdk::PluginRuntime::Process,
        bingo_plugin_host::manifest::Launch::Module(_) => bingo_sdk::PluginRuntime::Module,
    };
    let reason = if switched_off.contains(&found.name) {
        Some(SWITCHED_OFF.to_string())
    } else {
        bingo_plugin_host::runtime::runtime_of(
            &found.manifest,
            !switched_off.contains("bingo.mods"),
        )
        .err()
    };
    let grant = bingo_plugin_host::grant::grant(runtime, found.origin, None, &found.manifest.uses);
    PluginStatus {
        id: found.name,
        version: found.manifest.version,
        enabled: reason.is_none(),
        reason,
        execution: Some(bingo_sdk::PluginExecution {
            origin: found.origin,
            runtime,
            grant,
        }),
        from: match runtime {
            bingo_sdk::PluginRuntime::Process => bingo_plugin_rpc::ID,
            bingo_sdk::PluginRuntime::Module => "bingo.mods",
        }
        .into(),
    }
}

/// One line per plugin, with unavailable optional services in the reason column.
fn list_with_uses(
    listed: &[PluginStatus],
    composed: &[Box<dyn Plugin>],
    trust: &BTreeMap<String, bingo_sdk::PluginTrust>,
) -> String {
    let provided: BTreeSet<&str> = composed
        .iter()
        .filter(|plugin| {
            listed
                .iter()
                .any(|status| status.id == plugin.manifest().id && status.enabled)
        })
        .flat_map(|plugin| {
            plugin
                .manifest()
                .provides
                .iter()
                .filter_map(|key| key.strip_prefix("service:"))
        })
        .collect();
    listed
        .iter()
        .map(|status| {
            let mut line = row(status);
            let missing = missing_uses(status, composed, &provided);
            if !missing.is_empty() {
                line.push_str(&format!("missing used services: {}", missing.join(", ")));
            }
            let tier = if status.from == bingo_sdk::BUILT_IN {
                bingo_sdk::Tier::Full
            } else if let Some(execution) = &status.execution {
                execution.grant.tier
            } else {
                trust
                    .get(&status.id)
                    .map_or(bingo_sdk::Tier::Tighten, |grant| grant.tier)
            };
            line.push_str(&format!(
                "\t{}",
                match tier {
                    bingo_sdk::Tier::Tighten => "tighten",
                    bingo_sdk::Tier::Rewrite => "rewrite",
                    bingo_sdk::Tier::Full => "full",
                }
            ));
            let runtime =
                status
                    .execution
                    .as_ref()
                    .map_or("native", |execution| match execution.runtime {
                        bingo_sdk::PluginRuntime::Process => "process",
                        bingo_sdk::PluginRuntime::Module => "module",
                    });
            line.push_str(&format!("\t{runtime}"));
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn missing_uses<'a>(
    status: &PluginStatus,
    composed: &'a [Box<dyn Plugin>],
    provided: &BTreeSet<&str>,
) -> Vec<&'a str> {
    if !status.enabled {
        return Vec::new();
    }
    composed
        .iter()
        .find(|plugin| plugin.manifest().id == status.id)
        .into_iter()
        .flat_map(|plugin| plugin.manifest().uses.iter().copied())
        .filter(|key| !provided.contains(key))
        .collect()
}

fn row(status: &PluginStatus) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        status.id,
        status.version,
        state(status.enabled),
        status.reason.clone().unwrap_or_default()
    )
}

/// Write one switch into the user settings layer, or say why this is not one
/// to write. The refusals are `/plugins`'s own (ADR-0057 §3, §6).
fn switched(
    env: &Env,
    composed: &[Box<dyn Plugin>],
    listed: &[PluginStatus],
    name: &str,
    enabled: bool,
) -> Result<String, KernelError> {
    refuse(composed, listed, name)?;
    bingo_core::plugins::switch(env, name, enabled)
        .map_err(|e| KernelError::new(ErrorCode::Internal, e.to_string()))?;
    Ok(format!("{name} is {} at the next start.", state(enabled)))
}

fn refuse(
    composed: &[Box<dyn Plugin>],
    listed: &[PluginStatus],
    name: &str,
) -> Result<(), KernelError> {
    if !listed.iter().any(|status| status.id == name) {
        return Err(invalid(format!(
            "no plugin named `{name}`; `bingo plugins list` lists them"
        )));
    }
    let why = composed
        .iter()
        .map(|plugin| plugin.manifest())
        .find(|manifest| manifest.id == name)
        .and_then(needed);
    match why {
        Some(why) => Err(invalid(ignored(name, why))),
        None => Ok(()),
    }
}

fn invalid(message: String) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(id: &str, enabled: bool, reason: Option<&str>) -> PluginStatus {
        PluginStatus {
            id: id.into(),
            version: "0.1.0".into(),
            enabled,
            reason: reason.map(str::to_string),
            execution: None,
            from: bingo_sdk::BUILT_IN.into(),
        }
    }

    /// One line each, four columns, and a standing plugin's reason is empty
    /// rather than a word standing in for one.
    #[test]
    fn a_listing_is_one_tab_separated_line_per_plugin() {
        let said = list_with_uses(
            &[
                status("bingo.tools.web", true, None),
                status("wordcount", false, Some(SWITCHED_OFF)),
            ],
            &[],
            &BTreeMap::new(),
        );
        assert_eq!(
            said,
            format!(
                "bingo.tools.web\t0.1.0\ton\t\tfull\tnative\nwordcount\t0.1.0\toff\t{SWITCHED_OFF}\tfull\tnative"
            )
        );
    }

    /// The composition is what says a name exists, so a name nobody ships and
    /// nobody installed is refused before any file is touched.
    #[test]
    fn a_name_no_listing_knows_is_refused() {
        let listed = [status("bingo.tools.web", true, None)];
        let refused = refuse(&[], &listed, "bingo.tools.wb").expect_err("no such plugin");
        assert_eq!(refused.code, ErrorCode::InvalidInput);
        assert!(refused.message.contains("bingo plugins list"), "{refused}");
        refuse(&[], &listed, "bingo.tools.web").expect("a name the listing knows");
    }
}
