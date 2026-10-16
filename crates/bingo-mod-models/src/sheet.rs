use bingo_sdk::*;
#[derive(Clone)]
pub enum Sheet {
    Manage {
        provider: String,
        selected: usize,
        pending: Option<String>,
        confirm: bool,
    },
    SignIn {
        provider: String,
        selected: usize,
        pending: Option<String>,
    },
    Add {
        guess: crate::guess::Guess,
        selected: usize,
    },
}
impl Sheet {
    pub fn title(&self) -> String {
        match self {
            Self::Manage { provider, .. } => provider.clone(),
            Self::SignIn { provider, .. } => format!("sign in to {provider}"),
            Self::Add { .. } => "add a provider".into(),
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
pub fn lines(sheet: &Sheet, providers: &[CatalogEntry]) -> Vec<String> {
    let mut lines = vec![format!("model › {}", sheet.title()), String::new()];
    match sheet {
        Sheet::Manage {
            provider,
            selected,
            pending,
            confirm,
        } => append_manage(
            &mut lines,
            providers,
            provider,
            *selected,
            pending.is_some(),
            *confirm,
        ),
        Sheet::SignIn {
            selected, pending, ..
        } => append_sign_in(&mut lines, *selected, pending.as_deref()),
        Sheet::Add { guess, selected } => append_add(&mut lines, guess, *selected),
    }
    lines
}
fn append_sign_in(lines: &mut Vec<String>, selected: usize, pending: Option<&str>) {
    if let Some(id) = pending {
        lines.push(format!("   then switch to {id}"));
    }
    for (i, label) in [
        "open the browser",
        "use a code on another device",
        "paste a key or token",
    ]
    .iter()
    .enumerate()
    {
        lines.push(format!("{}{label}", mark(i == selected)));
    }
}
fn append_add(lines: &mut Vec<String>, guess: &crate::guess::Guess, selected: usize) {
    let address = if guess.address.is_empty() {
        "none · paste whose key this is"
    } else {
        guess.address.as_str()
    };
    let key = if guess.key.is_some() {
        "••••••••"
    } else {
        "none"
    };
    for (i, (label, value)) in [
        ("name", guess.name.as_str()),
        ("shape", guess.shape.as_str()),
        ("address", address),
        ("key", key),
        ("add provider", "then pick one of its models"),
    ]
    .iter()
    .enumerate()
    {
        lines.push(format!("{}{label:10} {value}", mark(i == selected)));
    }
}
fn mark(selected: bool) -> &'static str {
    if selected { " ❯ " } else { "   " }
}

fn append_manage(
    lines: &mut Vec<String>,
    providers: &[CatalogEntry],
    provider: &str,
    selected: usize,
    pending: bool,
    confirm: bool,
) {
    if let Some(p) = providers.iter().find(|p| p.id == provider) {
        lines.push(format!(
            "   address   {}",
            p.meta["address"]
                .as_str()
                .unwrap_or("managed by the provider")
        ));
        lines.push(format!(
            "   key       {}",
            p.meta["auth"]["from"].as_str().unwrap_or("none")
        ));
        lines.push(format!("   status    {}", crate::label::status(p)));
        lines.push(String::new());
        for (i, (_, label, enabled)) in actions(p, pending).iter().enumerate() {
            lines.push(format!(
                "{}{}{}",
                mark(i == selected),
                label,
                if *enabled {
                    ""
                } else {
                    " · from env; unset it in your shell"
                }
            ));
        }
        if confirm {
            lines.push(format!("Remove {provider} and its key?"));
        }
    }
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
