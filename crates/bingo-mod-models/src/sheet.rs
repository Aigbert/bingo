use bingo_sdk::*;
#[derive(Clone)]
pub enum Sheet {
    Manage {
        provider: String,
        selected: Option<String>,
        pending: Option<String>,
        confirm: bool,
    },
    SignIn {
        provider: String,
        selected: Option<String>,
        pending: Option<String>,
    },
}
impl Sheet {
    pub fn title(&self) -> String {
        match self {
            Self::Manage { provider, .. } => provider.clone(),
            Self::SignIn { provider, .. } => format!("sign in to {provider}"),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Anyway,
    Test,
    Address,
    Key,
    Refresh,
    Logout,
    Remove,
}
pub fn actions(provider: &CatalogEntry, pending: bool) -> Vec<(Operation, &'static str, bool)> {
    let mut actions = Vec::new();
    if pending {
        actions.push((Operation::Anyway, "use anyway", true));
    }
    actions.push((Operation::Test, "test the connection", true));
    if provider.meta["instance"] == true {
        actions.push((Operation::Address, "change the address", true));
    }
    actions.push((Operation::Key, "sign in / replace the key", true));
    actions.push((Operation::Refresh, "refresh the model list", true));
    actions.push((
        Operation::Logout,
        "sign out",
        provider.meta["auth"]["from"] != "env",
    ));
    if provider.meta["instance"] == true {
        actions.push((Operation::Remove, "remove provider", true));
    }
    actions
}
impl Operation {
    pub fn id(self) -> &'static str {
        match self {
            Self::Anyway => "anyway",
            Self::Test => "test",
            Self::Address => "address",
            Self::Key => "key",
            Self::Refresh => "refresh",
            Self::Logout => "logout",
            Self::Remove => "remove",
        }
    }
}
pub fn signin_rows() -> Vec<pick::Row> {
    [
        ("browser", "open the browser"),
        ("device", "use a code on another device"),
        ("paste", "paste a key or token"),
    ]
    .into_iter()
    .map(|(id, label)| pick::Row::new(id, pick::text(label, "muted")))
    .collect()
}
impl Sheet {
    pub fn selected_mut(&mut self) -> &mut Option<String> {
        match self {
            Self::Manage { selected, .. } | Self::SignIn { selected, .. } => selected,
        }
    }
    pub fn build(&self, providers: &[CatalogEntry]) -> bingo_sdk::sheet::Sheet {
        let mut sheet = bingo_sdk::sheet::Sheet {
            head: self.title(),
            fact: None,
            top: vec![],
            rows: vec![],
            selected: None,
            foot: vec![],
            pick: true,
        };
        match self {
            Self::SignIn {
                selected, pending, ..
            } => {
                sheet.selected = selected.clone();
                sheet.rows = signin_rows();
                if let Some(id) = pending {
                    sheet
                        .top
                        .push(pick::text(format!("then switch to {id}"), "dim"));
                }
            }
            Self::Manage {
                provider,
                selected,
                pending,
                confirm,
            } => {
                sheet.selected = selected.clone();
                if let Some(provider) = providers.iter().find(|p| &p.id == provider) {
                    sheet.top = details(provider);
                    sheet.rows = actions(provider, pending.is_some())
                        .into_iter()
                        .map(|(op, label, enabled)| {
                            let label = if enabled {
                                label.into()
                            } else {
                                format!("{label} · from env; unset it in your shell")
                            };
                            pick::Row::new(op.id(), pick::text(label, "muted"))
                        })
                        .collect();
                    if *confirm {
                        sheet.foot.push(pick::text(
                            format!("Remove {} and its key?", provider.id),
                            "amber",
                        ));
                    }
                }
            }
        }
        sheet
    }
}
fn details(provider: &CatalogEntry) -> Vec<View> {
    [
        (
            "address",
            provider.meta["address"]
                .as_str()
                .unwrap_or("managed by the provider")
                .to_owned(),
        ),
        (
            "key",
            provider.meta["auth"]["from"]
                .as_str()
                .unwrap_or("none")
                .to_owned(),
        ),
        ("status", crate::label::status(provider)),
    ]
    .into_iter()
    .map(|(label, value)| {
        pick::line([
            pick::pad(pick::text(label, "dim"), 10),
            pick::text(value, "muted"),
        ])
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_credentials_disable_signout_and_builtin_endpoints_cannot_be_removed() {
        let provider = CatalogEntry {
            id: "p".into(),
            label: "p".into(),
            meta: serde_json::json!({"auth":{"from":"env"},"instance":false}),
        };
        let actions = actions(&provider, false);
        assert!(
            actions
                .iter()
                .any(|(op, _, enabled)| *op == Operation::Logout && !enabled)
        );
        assert!(
            !actions
                .iter()
                .any(|(op, _, _)| matches!(op, Operation::Address | Operation::Remove))
        );
    }
}
