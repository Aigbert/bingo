use super::control::Control;
use crate::{pulled, tree::Tree};
use bingo_sdk::{
    Action, SessionId, SessionState,
    render::{Component, PullRequest},
    slot::{Shown, Slot},
};

#[derive(Debug)]
enum Site {
    Placed(String),
    Pulled {
        component: Component,
        request: String,
    },
}
#[derive(Debug)]
pub struct Binding {
    site: Site,
    control: Control,
    owner: String,
}
pub struct Resolution {
    pub action: Option<Action>,
    pub waiting: bool,
}

impl Binding {
    pub fn capture(state: &SessionState, memo: &pulled::Memo, action: &Action) -> Option<Self> {
        for pane in crate::pane::placed(state) {
            if let Shown::View(view) = &pane.view
                && let Some(control) = Control::bind(view, action)
            {
                return Some(Self::new(Site::Placed(pane.key), control, action));
            }
        }
        memo.drawings(&state.summary.id).find_map(|(key, view)| {
            Control::bind(view, action).map(|control| {
                Self::new(
                    Site::Pulled {
                        component: key.request.component,
                        request: key.request.request_id.clone(),
                    },
                    control,
                    action,
                )
            })
        })
    }

    fn new(site: Site, control: Control, action: &Action) -> Self {
        Self {
            site,
            control,
            owner: action.name.trim_end_matches(":ui").into(),
        }
    }

    /// `shown` is what the last frame pulled: a site it drew is still placed
    /// even while its memo is being rebuilt.
    pub fn resolve(
        &self,
        session: &SessionId,
        tree: &Tree,
        memo: &pulled::Memo,
        shown: &[pulled::Key],
    ) -> Resolution {
        let Some(state) = tree.state(session) else {
            return Resolution {
                action: None,
                waiting: false,
            };
        };
        match &self.site {
            Site::Placed(key) => Resolution {
                action: crate::pane::placed(state)
                    .into_iter()
                    .find(|pane| &pane.key == key)
                    .and_then(|pane| match pane.view {
                        Shown::View(view) => self.control.resolve(&view),
                        _ => None,
                    }),
                waiting: false,
            },
            Site::Pulled { component, request } => {
                let (view, waiting) = memo
                    .current(*component, request, state)
                    .unwrap_or_else(|| (None, still_pulled(state, shown, *component, request)));
                Resolution {
                    action: view.and_then(|view| self.control.resolve(view)),
                    waiting,
                }
            }
        }
    }

    pub fn notice(&self) -> String {
        format!("{}: {}", self.owner, bingo_sdk::view::STALE_CONTROL)
    }
}

/// Whether a pulled site is still on the screen or still asked for by a slot.
fn still_pulled(
    state: &SessionState,
    shown: &[pulled::Key],
    component: Component,
    request: &str,
) -> bool {
    let named =
        |pulled: &PullRequest| pulled.component == component && pulled.request_id == request;
    shown.iter().any(|key| {
        key.session == state.summary.id
            && key.request.component == component
            && key.request.request_id == request
    }) || bingo_sdk::slot::placed(state)
        .iter()
        .any(|placed| pulled_by(&placed.slot).is_some_and(named))
}

fn pulled_by(slot: &Slot) -> Option<&PullRequest> {
    match slot {
        Slot::Activity { view, .. }
        | Slot::Rail { view, .. }
        | Slot::Panel { view, .. }
        | Slot::Pane { view, .. } => match view {
            Shown::Pull { request } => Some(request),
            Shown::View(_) => None,
        },
        _ => None,
    }
}
