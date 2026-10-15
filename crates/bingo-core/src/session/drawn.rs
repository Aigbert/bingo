//! Which built-in drew each view a client presses keys on (ADR-0086 §7). The
//! host writes it as it draws; the actor reads it to vouch for what a key
//! answered. Nothing a client or a mod sends can write it.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bingo_sdk::Action;

/// Enough for every site a person can have open at once, several times over.
const KEPT: usize = 64;

/// A view a compiled-in mod drew itself, with the key handler it drew.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Drawn {
    pub(crate) plugin: String,
    pub(crate) keys: Action,
}

/// One session's drawings, newest last, by render request.
#[derive(Clone, Default)]
pub struct Drawings(Arc<Mutex<VecDeque<(String, Drawn)>>>);

impl Drawings {
    /// The latest drawing of a request replaces what was drawn before; a
    /// drawing nobody vouches for forgets it.
    pub(crate) fn record(&self, request: &str, drawn: Option<Drawn>) {
        let Ok(mut kept) = self.0.lock() else {
            return;
        };
        kept.retain(|(id, _)| id != request);
        if let Some(drawn) = drawn {
            kept.push_back((request.to_string(), drawn));
        }
        while kept.len() > KEPT {
            kept.pop_front();
        }
    }

    /// The built-in whose own drawing handed out this key handler: the same
    /// command for the same handler on the same row. A client adds only what
    /// was pressed.
    pub(crate) fn vouches(&self, action: &Action) -> Option<String> {
        let handler = action.args.get("handler").filter(|h| !h.is_null())?;
        let kept = self.0.lock().ok()?;
        kept.iter()
            .rev()
            .find(|(_, drawn)| {
                drawn.keys.name == action.name
                    && drawn.keys.args.get("handler") == Some(handler)
                    && drawn.keys.args.get("row") == action.args.get("row")
            })
            .map(|(_, drawn)| drawn.plugin.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn drawn(plugin: &str) -> Drawn {
        Drawn {
            plugin: plugin.into(),
            keys: Action {
                name: format!("{plugin}:ui"),
                args: json!({"handler":"r1","row":"a"}),
            },
        }
    }

    fn pressed(name: &str, handler: &str) -> Action {
        Action {
            name: name.into(),
            args: json!({"handler":handler,"row":"a","value":{"key":"enter"}}),
        }
    }

    #[test]
    fn only_the_handler_a_built_in_drew_is_vouched_for() {
        let drawings = Drawings::default();
        drawings.record("r1", Some(drawn("plugins")));
        assert_eq!(
            drawings.vouches(&pressed("plugins:ui", "r1")).as_deref(),
            Some("plugins")
        );
        assert_eq!(drawings.vouches(&pressed("plugins:ui", "r2")), None);
        assert_eq!(drawings.vouches(&pressed("other:ui", "r1")), None);
        drawings.record("r1", None);
        assert_eq!(drawings.vouches(&pressed("plugins:ui", "r1")), None);
    }

    /// A press that names a row the drawing did not show selected is not the
    /// drawing's own key.
    #[test]
    fn a_press_on_another_row_is_not_vouched_for() {
        let drawings = Drawings::default();
        drawings.record("r1", Some(drawn("plugins")));
        let mut moved = pressed("plugins:ui", "r1");
        moved.args["row"] = json!("b");
        assert_eq!(drawings.vouches(&moved), None);
        moved.args.as_object_mut().unwrap().remove("row");
        assert_eq!(drawings.vouches(&moved), None);
    }

    #[test]
    fn drawings_are_bounded() {
        let drawings = Drawings::default();
        for n in 0..KEPT + 8 {
            drawings.record(&format!("r{n}"), Some(drawn("plugins")));
        }
        assert_eq!(drawings.0.lock().unwrap().len(), KEPT);
    }
}
