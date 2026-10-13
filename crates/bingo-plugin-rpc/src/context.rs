//! A reverse request inherits only a live chain held by its own bridge.
use bingo_plugin_host::invocation::{self, CallingHook};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename = "HostCallContext")]
pub struct Context {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<CallingHook>,
}

#[derive(Clone, Default)]
pub(crate) struct Chains(Arc<Mutex<BTreeMap<String, Invocation>>>);
struct Invocation {
    calling: Vec<CallingHook>,
    rendering: bool,
    reads: BTreeMap<Option<usize>, bingo_plugin_host::state::ReadLog>,
}

impl Chains {
    pub fn hold(&self, chain: &str) -> Held {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).insert(
            chain.into(),
            Invocation {
                calling: invocation::current(),
                rendering: false,
                reads: BTreeMap::new(),
            },
        );
        Held {
            chains: self.clone(),
            chain: chain.into(),
        }
    }

    pub fn rendering(&self, context: &Context) -> bool {
        context
            .parent
            .as_ref()
            .and_then(|parent| {
                self.0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(parent)
                    .map(|invocation| invocation.rendering)
            })
            .unwrap_or(false)
    }

    pub fn reads(&self, context: &Context) -> bingo_plugin_host::state::ReadLog {
        let Some(parent) = &context.parent else {
            return Default::default();
        };
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(parent)
            .map(|invocation| invocation.reads.entry(context.hook).or_default().clone())
            .unwrap_or_default()
    }

    pub fn bind(&self, plugin: &str, context: Context) -> Result<Vec<CallingHook>, String> {
        let mut calling = match context.parent {
            Some(parent) => self
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&parent)
                .map(|invocation| invocation.calling.clone())
                .ok_or_else(|| {
                    "the parent hook chain is not live on this connection".to_string()
                })?,
            None => Vec::new(),
        };
        if let Some(hook) = context.hook {
            calling.push(CallingHook {
                plugin: plugin.into(),
                hook,
            });
        }
        Ok(calling)
    }
}

pub(crate) struct Held {
    chains: Chains,
    chain: String,
}
impl Held {
    pub fn render(&self) {
        if let Some(invocation) = self
            .chains
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.chain)
        {
            invocation.rendering = true;
        }
    }
    pub fn reads(&self) -> Vec<bingo_plugin_host::state::Read> {
        self.chains
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&self.chain)
            .map(|invocation| {
                invocation
                    .reads
                    .values()
                    .flat_map(|log| log.entries())
                    .collect()
            })
            .unwrap_or_default()
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        self.chains
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.chain);
    }
}

pub(crate) fn outgoing(mut params: serde_json::Value) -> serde_json::Value {
    let callers = invocation::current();
    if !callers.is_empty()
        && let Some(object) = params.as_object_mut()
    {
        object.insert("_bingo".into(), serde_json::json!({"callers":callers}));
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_reverse_call_inherits_its_parent_and_cannot_spoof_another_plugin() {
        let chains = Chains::default();
        let ancestor = CallingHook {
            plugin: "ancestor".into(),
            hook: 3,
        };
        let held = invocation::scope(vec![ancestor.clone()], async { chains.hold("chain") }).await;
        let context = || Context {
            parent: Some("chain".into()),
            hook: Some(1),
            callers: vec![CallingHook {
                plugin: "victim".into(),
                hook: 0,
            }],
        };
        assert_eq!(
            chains.bind("caller", context()).unwrap(),
            vec![
                ancestor,
                CallingHook {
                    plugin: "caller".into(),
                    hook: 1
                }
            ]
        );
        assert!(Chains::default().bind("caller", context()).is_err());
        drop(held);
        assert!(chains.bind("caller", context()).is_err());
    }

    #[tokio::test]
    async fn concurrent_chains_keep_distinct_ancestry() {
        let chains = Chains::default();
        let first = invocation::from("first", 0, async { chains.hold("first") }).await;
        let second = chains.hold("second");
        let context = Context {
            parent: Some("second".into()),
            hook: Some(2),
            callers: vec![],
        };
        assert_eq!(
            chains.bind("caller", context).unwrap(),
            vec![CallingHook {
                plugin: "caller".into(),
                hook: 2
            }]
        );
        drop((first, second));
    }
}
