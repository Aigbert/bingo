//! Slash commands. One registry serves dispatch, the catalog and help; a
//! surface completes a command's argument from the `ArgSpec` its catalogue
//! entry carries, never by asking the command. The session actor parses `/name args`, `!line` and `Input::Action`,
//! runs the command on its own task and answers with an `IntentAck` whose
//! `Applied.result` is `{"message"}`, `{"view"}` or `{"item"}` (ADR-0008).

use std::path::PathBuf;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::KernelError;
use crate::event::ItemBody;
use crate::host::HostHandle;
use crate::ids::SessionId;
use crate::view::View;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// A one-line description for the command menu; argument syntax belongs in `args`.
    pub hint: String,
    pub args: ArgSpec,
    /// May run while a turn is busy (read-only commands).
    #[serde(default)]
    pub instant: bool,
    pub family: String,
}

impl CommandSpec {
    pub fn ui(plugin: &str) -> Self {
        Self {
            keys: vec![],
            name: format!("{plugin}:ui"),
            aliases: vec![],
            hint: String::new(),
            args: ArgSpec::None,
            instant: true,
            hidden: true,
            family: "ui".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ArgSpec {
    None,
    /// An argument followed by optional flags, completed from the same spec.
    Options {
        args: Box<ArgSpec>,
        flags: Vec<String>,
    },
    Free {
        hint: String,
    },
    /// Completed from a catalog kind the command validates against.
    Catalog {
        source: String,
    },
    /// One of the words the command lists itself, in the order it offers
    /// them. A surface completes from the list; the kernel carries it.
    ///
    /// `then` says what the word after one of `values` is — the name
    /// `/plugins enable` takes, the server `/mcp reconnect` takes — and is
    /// absent for a command whose argument ends at the first word, so every
    /// spec written before it reads the same (ADR-0008 §6a).
    Words {
        /// Words among `values` that end the argument; `then` does not apply.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        alone: Vec<String>,
        values: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        then: Option<Box<ArgSpec>>,
    },
}

#[derive(Clone)]
pub struct CommandContext {
    pub session: SessionId,
    pub cwd: PathBuf,
    pub host: HostHandle,
    /// Cancelled when the person stops this command or the session closes.
    pub cancel: crate::CancellationToken,
}

impl std::fmt::Debug for CommandContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandContext")
            .field("session", &self.session)
            .field("cwd", &self.cwd)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandOutcome {
    /// A site key chose an answer; the surface decides whether to send it.
    Answer {
        answer: crate::Answer,
    },
    Handled {
        handled: bool,
    },
    Applied {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    View {
        view: View,
    },
    /// Becomes a turn, submitted with the command's own intent and origin.
    Prompt {
        text: String,
    },
    /// One completed item the kernel records in the transcript (a shell
    /// line's output, a login's receipt); the ack carries its id.
    Record {
        body: ItemBody,
    },
}

#[async_trait]
pub trait Command: Send + Sync {
    fn spec(&self) -> CommandSpec;

    /// Whether this invocation may run beside the current turn. Implementations
    /// inspect the supplied state without calling back into its session actor.
    async fn is_instant(&self, _args: &str, _state: &crate::SessionState) -> bool {
        self.spec().instant
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn optional_flags_wrap_an_existing_argument_contract() {
        let wire = serde_json::json!({"kind":"options","args":{"kind":"catalog","source":"models"},"flags":["--keep"]});
        let args: ArgSpec = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(args).unwrap(), wire);
    }

    #[test]
    fn a_frame_without_alone_keeps_its_following_argument() {
        let wire = json!({
            "name": "plugins", "hint": "plugins", "family": "session",
            "args": {
                "kind": "words", "values": ["enable"],
                "then": { "kind": "catalog", "source": "plugins" }
            }
        });
        let spec: CommandSpec = serde_json::from_value(wire).expect("old frame");
        let ArgSpec::Words { alone, then, .. } = spec.args else {
            panic!("word arguments");
        };
        assert!(alone.is_empty());
        assert_eq!(
            then.as_deref(),
            Some(&ArgSpec::Catalog {
                source: "plugins".into()
            })
        );
    }

    #[test]
    fn words_that_end_the_argument_survive_the_wire() {
        let wire = json!({
            "kind": "words", "values": ["enable", "reload"],
            "alone": ["reload"],
            "then": { "kind": "catalog", "source": "plugins" }
        });
        let args: ArgSpec = serde_json::from_value(wire.clone()).expect("word arguments");
        assert_eq!(serde_json::to_value(args).expect("serialises"), wire);
    }

    /// The shape a surface and a plugin both read: the variant is its `kind`,
    /// the words are `values`, in the command's own order. A word list that
    /// says nothing about the word after it writes nothing down, so a spec
    /// from before `then` and a spec without one are the same bytes.
    #[test]
    fn a_word_list_crosses_the_wire_as_its_kind_and_its_values() {
        let args = ArgSpec::Words {
            alone: vec![],
            values: vec!["off".into(), "low".into()],
            then: None,
        };
        let wire = serde_json::to_value(&args).expect("serialises");
        assert_eq!(wire, json!({ "kind": "words", "values": ["off", "low"] }));
        let back: ArgSpec = serde_json::from_value(wire).expect("deserialises");
        assert_eq!(back, args);
    }

    /// A verb that takes a name carries the name's own spec inside its own,
    /// and a reader that has never heard of one reads the words as before.
    #[test]
    fn a_verb_carries_the_spec_of_the_word_after_it() {
        let args = ArgSpec::Words {
            alone: vec![],
            values: vec!["enable".into()],
            then: Some(Box::new(ArgSpec::Catalog {
                source: "plugins".into(),
            })),
        };
        let wire = serde_json::to_value(&args).expect("serialises");
        assert_eq!(
            wire,
            json!({
                "kind": "words",
                "values": ["enable"],
                "then": { "kind": "catalog", "source": "plugins" },
            })
        );
        let back: ArgSpec = serde_json::from_value(wire).expect("deserialises");
        assert_eq!(back, args);
    }
}

#[cfg(test)]
mod hidden_contract {
    use super::*;
    #[test]
    fn hidden_defaults_false_and_survives_the_wire_when_true() {
        let old = serde_json::json!({"name":"counter:ui","hint":"","args":{"kind":"none"},"family":"ui","instant":true});
        let spec: CommandSpec = serde_json::from_value(old.clone()).unwrap();
        assert!(serde_json::to_value(spec).unwrap().get("hidden").is_none());
        let mut hidden = old;
        hidden["hidden"] = serde_json::json!(true);
        let spec: CommandSpec = serde_json::from_value(hidden).unwrap();
        assert_eq!(serde_json::to_value(spec).unwrap()["hidden"], true);
    }
}

#[cfg(test)]
mod key_contract {
    use super::*;
    use serde_json::json;
    #[test]
    fn default_keys_are_additive_and_handled_is_an_explicit_result() {
        let old = json!({"name":"hello","hint":"Hello","args":{"kind":"none"},"family":"fixture"});
        let mut spec: CommandSpec = serde_json::from_value(old).unwrap();
        assert!(spec.keys.is_empty());
        assert!(serde_json::to_value(&spec).unwrap().get("keys").is_none());
        spec.keys.push("ctrl+x".into());
        assert_eq!(
            serde_json::from_value::<CommandSpec>(serde_json::to_value(&spec).unwrap()).unwrap(),
            spec
        );
        let wire = json!({"kind":"handled","handled":false});
        assert_eq!(
            serde_json::to_value(serde_json::from_value::<CommandOutcome>(wire.clone()).unwrap())
                .unwrap(),
            wire
        );
    }
}

/// Provider arguments can contain credentials even when dispatch will reject them.
/// Use this only for display and persistence; dispatch keeps the original line.
pub fn recorded_line(line: &str) -> &str {
    if line.split_whitespace().next() == Some("/provider") {
        "/provider [redacted]"
    } else {
        line
    }
}
