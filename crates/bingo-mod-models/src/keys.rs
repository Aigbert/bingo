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
        let press_value = if args["event"] == "close" {
            bingo_sdk::sheet::Press::Key("esc".into())
        } else {
            let Some(press) = bingo_sdk::sheet::pressed(&args) else {
                return Ok(handled());
            };
            press
        };
        let result = tokio::select! {
            _=cx.cancel.cancelled()=>Err(crate::error("picker action cancelled")),
            result=press(&mut picker,&press_value,cx)=>result,
        };
        let close = match result {
            Ok(close) => close,
            Err(error) => {
                crate::pins::say(cx, error.message).await?;
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
    press: &bingo_sdk::sheet::Press,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let key = press.key().unwrap_or("enter");
    if key == "esc" {
        return Ok(back(picker));
    }
    if picker.sheet.is_some() {
        return sheet_key(picker, press, cx).await;
    }
    if !matches!(key, "up" | "down" | "left" | "right" | "enter") {
        return Ok(false);
    }
    let models = cx.host.catalog(CatalogKind::Models).await?.entries;
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let rows = crate::rows::rows(&models, &picker.props.filter, picker.all);
    let key = if let Some(id) = press.picked() {
        if !rows.iter().any(|row| row.key() == id) {
            return Ok(false);
        }
        picker.props.selected = Some(id.into());
        "enter"
    } else {
        key
    };
    if key == "enter"
        && let Some(id) = unlisted(picker, &rows, cx).await?
    {
        return use_unlisted(picker, &id, cx).await;
    }
    match key {
        "up" | "down" => step(picker, &rows, key == "down"),
        _ => {
            if let Some(at) = acted_on(picker, &rows) {
                return act(picker, key, &rows[at], &models, &providers, cx).await;
            }
        }
    }
    Ok(false)
}
/// Moves from where the cursor is drawn to the next row that can be chosen.
fn step(picker: &mut Picker, rows: &[Row], forward: bool) {
    let rows = rows
        .iter()
        .map(|row| pick::Row::new(row.key(), pick::text("", "fg")))
        .collect::<Vec<_>>();
    picker.props.selected = pick::moved(&rows, picker.props.selected.as_deref(), forward);
}
/// A new filter puts the cursor on its first row; keeping that row by name
/// holds the cursor there while probes add rows above it.
pub(crate) fn keep_cursor(picker: &mut Picker, rows: &[Row]) {
    if picker.props.selected.is_none() {
        picker.props.selected = crate::rows::cursor(rows, None).map(|i| rows[i].key());
    }
}
/// A key acts on the row the picker keeps selected, never on whatever a
/// redraw now puts in its place. A row that is gone takes no action; the
/// cursor lands on the first row instead.
fn acted_on(picker: &mut Picker, rows: &[Row]) -> Option<usize> {
    let at = match picker.props.selected.as_deref() {
        Some(key) => crate::rows::named(rows, key),
        None => crate::rows::cursor(rows, None),
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
    let efforts = crate::card::levels(model);
    if efforts.is_empty() {
        picker.props.effort = None;
        return;
    }
    let current = crate::card::clamp(model, picker.props.effort);
    let index = efforts.iter().position(|e| *e == current).unwrap_or(0);
    let index = if forward {
        (index + 1).min(efforts.len() - 1)
    } else {
        index.saturating_sub(1)
    };
    picker.props.effort = efforts[index];
}
async fn activate(
    picker: &mut Picker,
    row: &Row,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    match row {
        Row::Model(id) => {
            let Some(model) = models.iter().find(|m| &m.id == id) else {
                return Ok(false);
            };
            if model.meta["chat"] == false {
                crate::pins::say(cx, "This model can't hold a chat.".into()).await?;
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
                        selected: None,
                        pending: Some(id.clone()),
                        confirm: false,
                    });
                    Ok(false)
                }
                crate::label::Reach::Unchecked => check_then_use(picker, model, provider, cx).await,
                crate::label::Reach::Ready => choose(picker, model, cx).await,
            }
        }
    }
}
fn signin(provider: &CatalogEntry, pending: Option<String>) -> Sheet {
    Sheet::SignIn {
        provider: provider.id.clone(),
        selected: Some(
            if provider.meta["shape"] == "codex" {
                "browser"
            } else {
                "paste"
            }
            .into(),
        ),
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
        cx.surface,
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
            cx.surface,
        )
        .await?;
    }
    let chosen = format!(
        "{} {}",
        model
            .id
            .split_once('/')
            .map_or(model.id.as_str(), |(_, id)| id),
        Effort::word(crate::card::clamp(model, picker.props.effort))
    );
    crate::pins::announce(cx, vec![chosen.into()], Tone::Good).await?;
    Ok(true)
}
/// The model the filter names when nothing listed matches it.
async fn unlisted(
    picker: &Picker,
    rows: &[Row],
    cx: &CommandContext,
) -> Result<Option<String>, KernelError> {
    if picker.props.filter.is_empty() || rows.iter().any(|r| r.model().is_some()) {
        return Ok(None);
    }
    let attached = crate::bridge::attach(&cx.host, &cx.session).await?;
    let current = crate::state::chat(&attached.snapshot).current;
    Ok(crate::rows::unlisted(rows, &picker.props.filter, &current))
}
/// Nothing listed matches, so the filter is taken as the model's name and the
/// kernel decides. The id names its provider: `/model` would take a bare word
/// as a filter and open the picker again.
async fn use_unlisted(picker: &Picker, id: &str, cx: &CommandContext) -> Result<bool, KernelError> {
    let keep = if picker.keep() { " --keep" } else { "" };
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "model",
        &format!("{id}{keep}"),
        None,
        cx.surface,
    )
    .await?;
    Ok(true)
}
async fn sheet_key(
    picker: &mut Picker,
    press: &bingo_sdk::sheet::Press,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let Some(mut sheet) = picker.sheet.take() else {
        return Ok(false);
    };
    let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
    let rows = sheet.build(&providers).rows;
    if let Some(id) = press.picked() {
        if !rows.iter().any(|row| row.id == id) {
            picker.sheet = Some(sheet);
            return Ok(false);
        }
        *sheet.selected_mut() = Some(id.into());
    }
    let key = press.key().unwrap_or("enter");
    let result = match &mut sheet {
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
    selected: &mut Option<String>,
    pending: &Option<String>,
    confirm: &mut bool,
    key: &str,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let actions = crate::sheet::actions(provider, pending.is_some());
    let rows = actions
        .iter()
        .map(|(op, label, _)| pick::Row::new(op.id(), pick::text(*label, "fg")))
        .collect::<Vec<_>>();
    crate::sheet_actions::navigate(selected, key, &rows);
    if key != "enter" {
        return Ok(false);
    }
    let id = &rows[pick::selected(&rows, selected.as_deref())].id;
    let Some((op, _, enabled)) = actions.iter().find(|(op, _, _)| op.id() == id) else {
        return Ok(false);
    };
    if !enabled {
        crate::pins::say(cx, "Unset the environment credential in your shell.".into()).await?;
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
                cx.surface,
            )
            .await?;
            return Ok(true);
        }
        Operation::Address => change_address(provider, cx).await?,
        Operation::Logout => {
            crate::bridge::run(
                &cx.host,
                &cx.session,
                "logout",
                &provider.id,
                None,
                cx.surface,
            )
            .await?
        }
        Operation::Test | Operation::Refresh => {
            crate::bridge::run(
                &cx.host,
                &cx.session,
                "models",
                &format!("refresh {}", provider.id),
                None,
                cx.surface,
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
        cx.surface,
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
        selected: None,
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
            cx.surface,
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

#[cfg(test)]
mod tests;
