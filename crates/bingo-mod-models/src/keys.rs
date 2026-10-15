use crate::{
    rows::Row,
    sheet::{Operation, Sheet},
    state::Picker,
};
use bingo_sdk::*;
use serde_json::Value;
pub struct Handler(pub crate::Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("bingo.mod.models")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        let Some(mut picker) = self
            .0
            .lock()
            .map_err(crate::error)?
            .get(&cx.session)
            .cloned()
        else {
            return Ok(handled());
        };
        if args["handler"] != picker.id {
            return Ok(handled());
        }
        let key = if args["event"] == "close" {
            "esc"
        } else {
            args["value"]["key"].as_str().unwrap_or_default()
        };
        let result = tokio::select! {
            _=cx.cancel.cancelled()=>Err(crate::error("picker action cancelled")),
            result=press(&mut picker,key,args["value"]["row"].as_str(),cx)=>result,
        };
        let close = match result {
            Ok(close) => close,
            Err(error) => {
                picker.message = Some(error.message);
                false
            }
        };
        if close {
            self.0.lock().map_err(crate::error)?.remove(&cx.session);
            crate::state::close(&cx.host, &cx.session).await?;
        } else {
            self.0
                .lock()
                .map_err(crate::error)?
                .insert(cx.session.clone(), picker.clone());
            crate::state::publish(&cx.host, &cx.session, &picker).await?;
        }
        Ok(handled())
    }
}
fn handled() -> CommandOutcome {
    CommandOutcome::Handled { handled: true }
}
async fn press(
    picker: &mut Picker,
    key: &str,
    drawn: Option<&str>,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    picker.message = None;
    if key == "esc" {
        return Ok(back(picker));
    }
    if picker.sheet.is_some() {
        return sheet_key(picker, key, cx).await;
    }
    if key == "+" || key == "shift++" {
        return add(picker, cx).await;
    }
    if edit(picker, key) {
        return Ok(false);
    }
    let models = cx.host.catalog(CatalogKind::Models).await?.entries;
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let rows = crate::rows::rows(&models, &providers, &picker.props.filter, picker.all);
    match key {
        "up" | "down" => step(picker, &rows, key == "down"),
        "enter" if crate::rows::unlisted(&rows, &picker.props.filter) => {
            return use_unlisted(picker, cx).await;
        }
        _ => {
            if let Some(at) = acted_on(picker, &rows, drawn) {
                return act(picker, key, &rows[at], &models, &providers, cx).await;
            }
        }
    }
    Ok(false)
}
/// Keys that change the filter or the scope, whatever row is under the cursor.
fn edit(picker: &mut Picker, key: &str) -> bool {
    let text = key.strip_prefix("shift+").unwrap_or(key);
    match key {
        "tab" => {
            picker.props.scope = if picker.keep() {
                component::ModelScope::Session
            } else {
                component::ModelScope::Keep
            }
        }
        "backspace" => {
            picker.props.filter.pop();
            picker.props.selected = None;
        }
        _ if text.chars().count() == 1 && text != "*" => {
            picker.props.filter.push_str(text);
            picker.props.selected = None;
        }
        _ => return false,
    }
    true
}
/// Moves from where the cursor is drawn to the next row that can be chosen.
fn step(picker: &mut Picker, rows: &[Row], forward: bool) {
    let Some(at) = crate::rows::cursor(rows, picker.props.selected.as_deref()) else {
        return;
    };
    let next = if forward {
        (at + 1..rows.len()).find(|i| rows[*i].selectable())
    } else {
        (0..at).rev().find(|i| rows[*i].selectable())
    };
    picker.props.selected = Some(rows[next.unwrap_or(at)].key());
}
/// The row a key acts on is the one drawn when it was pressed. A row that is
/// gone takes no action; the cursor goes back to the first row instead.
fn acted_on(picker: &mut Picker, rows: &[Row], drawn: Option<&str>) -> Option<usize> {
    let at = match drawn {
        Some(key) => crate::rows::named(rows, key),
        None => crate::rows::cursor(rows, picker.props.selected.as_deref()),
    };
    let kept = at.or_else(|| crate::rows::cursor(rows, None));
    picker.props.selected = kept.map(|i| rows[i].key());
    at
}
async fn act(
    picker: &mut Picker,
    key: &str,
    row: &Row,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    match key {
        "left" | "right" => dial(picker, row, models, key == "right"),
        "*" | "shift+*" => {
            if let Some(id) = row.model() {
                pin(cx, id, models).await?;
            }
        }
        "enter" => return activate(picker, row, models, providers, cx).await,
        _ => {}
    }
    Ok(false)
}
fn dial(picker: &mut Picker, row: &Row, models: &[CatalogEntry], forward: bool) {
    let Some(model) = row
        .model()
        .and_then(|id| models.iter().find(|m| m.id == id))
    else {
        return;
    };
    let efforts = crate::card::efforts(model);
    if efforts.is_empty() {
        picker.props.effort = None;
        return;
    }
    let current = crate::card::clamp(model, picker.props.effort);
    let index = efforts
        .iter()
        .position(|e| Some(*e) == current)
        .unwrap_or(0);
    let index = if forward {
        (index + 1).min(efforts.len() - 1)
    } else {
        index.saturating_sub(1)
    };
    picker.props.effort = Some(efforts[index]);
}
async fn pin(cx: &CommandContext, id: &str, models: &[CatalogEntry]) -> Result<(), KernelError> {
    let pinned = models
        .iter()
        .any(|m| m.id == id && m.meta["pinned"] == true);
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "model",
        &format!("{} {id}", if pinned { "unpin" } else { "pin" }),
        None,
    )
    .await
}
async fn activate(
    picker: &mut Picker,
    row: &Row,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    match row {
        Row::Add => add(picker, cx).await,
        Row::Hidden(_, _) => {
            picker.all = true;
            Ok(false)
        }
        Row::Provider(id) => {
            picker.sheet = Some(Sheet::Manage {
                provider: id.clone(),
                selected: 0,
                pending: None,
                confirm: false,
            });
            Ok(false)
        }
        Row::Pinned(id) | Row::Model(id) => {
            let Some(model) = models.iter().find(|m| &m.id == id) else {
                return Ok(false);
            };
            if model.meta["chat"] == false {
                picker.message = Some("This model can't hold a chat.".into());
                return Ok(false);
            }
            let Some(provider) = providers.iter().find(|p| model.meta["provider"] == p.id) else {
                return Ok(false);
            };
            match crate::label::reach(provider) {
                crate::label::Reach::Missing => {
                    picker.sheet = Some(signin(provider, Some(id.clone())));
                    Ok(false)
                }
                crate::label::Reach::Down => {
                    picker.sheet = Some(Sheet::Manage {
                        provider: provider.id.clone(),
                        selected: 0,
                        pending: Some(id.clone()),
                        confirm: false,
                    });
                    Ok(false)
                }
                crate::label::Reach::Unchecked => check_then_use(picker, model, provider, cx).await,
                crate::label::Reach::Ready => choose(picker, model, cx).await,
            }
        }
        _ => Ok(false),
    }
}
fn signin(provider: &CatalogEntry, pending: Option<String>) -> Sheet {
    Sheet::SignIn {
        provider: provider.id.clone(),
        selected: if provider.meta["shape"] == "codex" {
            0
        } else {
            2
        },
        pending,
    }
}
pub(crate) async fn choose(
    picker: &Picker,
    model: &CatalogEntry,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let keep = if picker.keep() { " --keep" } else { "" };
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "model",
        &format!("{}{keep}", model.id),
        None,
    )
    .await?;
    if model.meta["efforts"].is_array() {
        crate::bridge::run(
            &cx.host,
            &cx.session,
            "think",
            &format!(
                "{}{keep}",
                Effort::word(crate::card::clamp(model, picker.props.effort))
            ),
            None,
        )
        .await?;
    }
    Ok(true)
}
/// Nothing listed matches, so the filter is taken as the model's name and the kernel decides.
async fn use_unlisted(picker: &Picker, cx: &CommandContext) -> Result<bool, KernelError> {
    let keep = if picker.keep() { " --keep" } else { "" };
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "model",
        &format!("{}{keep}", picker.props.filter),
        None,
    )
    .await?;
    Ok(true)
}
async fn add(picker: &mut Picker, cx: &CommandContext) -> Result<bool, KernelError> {
    let Some(value) = crate::bridge::paste(
        &cx.host,
        &cx.session,
        "add a provider · paste a key or server address",
    )
    .await?
    else {
        return Ok(false);
    };
    let names = cx
        .host
        .catalog(CatalogKind::Providers)
        .await?
        .entries
        .into_iter()
        .map(|p| p.id)
        .chain(["openai".into(), "anthropic".into(), "codex".into()])
        .collect::<Vec<_>>();
    let guess = crate::guess::guess(&value, &names)
        .ok_or_else(|| crate::error("Use an API key or an http(s) server address."))?;
    // An unknown vendor opens on its address, the one thing still to say.
    let selected = if guess.address.is_empty() { 2 } else { 0 };
    picker.sheet = Some(Sheet::Add { guess, selected });
    Ok(false)
}
async fn sheet_key(
    picker: &mut Picker,
    key: &str,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let Some(mut sheet) = picker.sheet.take() else {
        return Ok(false);
    };
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let result = match &mut sheet {
        Sheet::Add { guess, selected } => {
            crate::sheet_actions::add_key(guess, selected, key, picker, cx).await
        }
        Sheet::SignIn {
            provider,
            selected,
            pending,
        } => crate::sheet_actions::signin_key(provider, selected, pending, key, picker, cx).await,
        Sheet::Manage {
            provider,
            selected,
            pending,
            confirm,
        } => {
            let p = providers
                .iter()
                .find(|p| &p.id == provider)
                .ok_or_else(|| crate::error("provider no longer exists"))?;
            manage_key(p, selected, pending, confirm, key, picker, cx).await
        }
    };
    if picker.sheet.is_none() && !matches!(result, Ok(true)) {
        picker.sheet = Some(sheet);
    }
    result
}
async fn manage_key(
    provider: &CatalogEntry,
    selected: &mut usize,
    pending: &Option<String>,
    confirm: &mut bool,
    key: &str,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let actions = crate::sheet::actions(provider, pending.is_some());
    crate::sheet_actions::navigate(selected, key, actions.len());
    if key != "enter" {
        return Ok(false);
    }
    let Some((op, _, enabled)) = actions.get(*selected) else {
        return Ok(false);
    };
    if !enabled {
        picker.message = Some("Unset the environment credential in your shell.".into());
        return Ok(false);
    }
    match op {
        Operation::Key => picker.sheet = Some(signin(provider, pending.clone())),
        Operation::Anyway => {
            if let Some(id) = pending {
                return crate::sheet_actions::choose_id(picker, id, cx).await;
            }
        }
        Operation::Remove if !*confirm => *confirm = true,
        Operation::Remove => {
            crate::bridge::run(
                &cx.host,
                &cx.session,
                "provider",
                &format!("remove {}", provider.id),
                None,
            )
            .await?;
            return Ok(true);
        }
        Operation::Address => change_address(provider, cx).await?,
        Operation::Logout => {
            crate::bridge::run(&cx.host, &cx.session, "logout", &provider.id, None).await?
        }
        Operation::Test | Operation::Refresh => {
            crate::bridge::run(
                &cx.host,
                &cx.session,
                "models",
                &format!("refresh {}", provider.id),
                None,
            )
            .await?
        }
    }
    Ok(false)
}

async fn check_then_use(
    picker: &mut Picker,
    model: &CatalogEntry,
    provider: &CatalogEntry,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "models",
        &format!("refresh {}", provider.id),
        None,
    )
    .await?;
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let checked = providers
        .iter()
        .find(|p| p.id == provider.id)
        .ok_or_else(|| crate::error("provider no longer exists"))?;
    if crate::label::reach(checked) == crate::label::Reach::Ready {
        return choose(picker, model, cx).await;
    }
    picker.sheet = Some(Sheet::Manage {
        provider: provider.id.clone(),
        selected: 0,
        pending: Some(model.id.clone()),
        confirm: false,
    });
    Ok(false)
}

async fn change_address(provider: &CatalogEntry, cx: &CommandContext) -> Result<(), KernelError> {
    if let Some(address) =
        crate::bridge::paste(&cx.host, &cx.session, "new provider address").await?
    {
        let guess = crate::guess::guess(&address, &[])
            .filter(|g| g.key.is_none())
            .ok_or_else(|| crate::error("Use an http(s) address without credentials."))?;
        crate::bridge::run(
            &cx.host,
            &cx.session,
            "provider",
            &format!(
                "set {} url {}",
                provider.id,
                crate::guess::wire_address(
                    provider.meta["shape"].as_str().unwrap_or_default(),
                    &guess.address
                )
            ),
            None,
        )
        .await?;
    }
    Ok(())
}

fn back(picker: &mut Picker) -> bool {
    if let Some(Sheet::Manage { confirm, .. }) = &mut picker.sheet
        && *confirm
    {
        *confirm = false;
        return false;
    }
    picker.sheet.take().is_none()
}
