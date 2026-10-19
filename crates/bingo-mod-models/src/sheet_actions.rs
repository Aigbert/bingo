use crate::state::Picker;
use bingo_sdk::*;
pub fn navigate(selected: &mut Option<String>, key: &str, rows: &[pick::Row]) {
    if matches!(key, "up" | "down") {
        *selected = pick::moved(rows, selected.as_deref(), key == "down");
    }
}
pub async fn choose_id(
    picker: &Picker,
    id: &str,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let model = cx
        .host
        .catalog(CatalogKind::Models)
        .await?
        .entries
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| crate::error("model is no longer listed"))?;
    crate::keys::choose(picker, &model, cx).await
}
pub async fn signin_key(
    provider: &str,
    selected: &mut Option<String>,
    pending: &Option<String>,
    key: &str,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let rows = crate::sheet::signin_rows();
    navigate(selected, key, &rows);
    if key != "enter" {
        return Ok(false);
    }
    let method = &rows[pick::selected(&rows, selected.as_deref())].id;
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "login",
        &format!("{provider} {method}"),
        None,
        cx.surface,
    )
    .await?;
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "models",
        &format!("refresh {provider}"),
        None,
        cx.surface,
    )
    .await?;
    if let Some(id) = pending {
        return choose_id(picker, id, cx).await;
    }
    picker.sheet = Some(crate::sheet::Sheet::Manage {
        provider: provider.into(),
        selected: None,
        pending: None,
        confirm: false,
    });
    Ok(false)
}
