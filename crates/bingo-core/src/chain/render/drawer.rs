//! Who drew a view: the link that answered it rather than passing on what
//! the rest of the chain answered.
use super::*;
use crate::session::Drawn;

/// One link's answer beside what the rest of the chain had answered it.
pub(super) struct Answered {
    pub(super) plugin: String,
    pub(super) compiled: bool,
    pub(super) answer: serde_json::Value,
    pub(super) inherited: Option<serde_json::Value>,
}

/// Answers in the order links finished: the innermost first.
#[derive(Clone, Default)]
pub(super) struct Trace(Arc<Mutex<Vec<Answered>>>);

impl Trace {
    pub(super) fn push(&self, answered: Answered) {
        if let Ok(mut trace) = self.0.lock() {
            trace.push(answered);
        }
    }

    /// A compiled-in mod that drew this view itself vouches for the keys it
    /// drew, and only for its own; a view anyone else drew or reshaped
    /// vouches for nothing.
    pub(super) fn drawn(&self, view: &View) -> Option<Drawn> {
        let View::Box { layout, .. } = view else {
            return None;
        };
        let keys = layout.on_key.clone()?;
        let trace = self.0.lock().ok()?;
        let drawer = trace.iter().find(|answered| {
            same(&answered.answer, view)
                && !answered.inherited.as_ref().is_some_and(|i| same(i, view))
        })?;
        (drawer.compiled && keys.name == format!("{}:ui", drawer.plugin)).then(|| Drawn {
            plugin: drawer.plugin.clone(),
            keys,
        })
    }
}

fn same(value: &serde_json::Value, view: &View) -> bool {
    serde_json::from_value::<View>(value.clone()).is_ok_and(|answered| &answered == view)
}
