use crate::{rows::Row, state::Picker};
use bingo_sdk::*;

pub async fn toggle(
    picker: &mut Picker,
    cx: &CommandContext,
    id: &str,
    models: &[CatalogEntry],
) -> Result<(), KernelError> {
    let model = models.iter().find(|m| m.id == id);
    let pinned = model.is_some_and(|m| m.meta["pinned"] == true);
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "model",
        &format!("{} {id}", if pinned { "unpin" } else { "pin" }),
        None,
    )
    .await?;
    if pinned {
        picker.all |= model.is_some_and(crate::rows::hidden);
        picker.props.selected = Some(Row::Model(id.into()).key());
    }
    let model = id.split_once('/').map_or(id, |(_, model)| model);
    announce(
        cx,
        format!("{} {model}", if pinned { "unpinned" } else { "pinned" }),
    )
    .await
}

pub(crate) async fn announce(cx: &CommandContext, text: String) -> Result<(), KernelError> {
    let until = (std::time::SystemTime::now() + std::time::Duration::from_millis(2800))
        .try_into()
        .map_err(crate::error)?;
    cx.host
        .signal(
            &cx.session,
            "bingo.mod.models",
            "pin",
            slot::with(
                serde_json::json!({}),
                [slot::Slot::Toast {
                    text,
                    until,
                    alert: false,
                }],
            ),
        )
        .await
}
