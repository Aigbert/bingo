use crate::{rows::Row, state::Picker};
use bingo_sdk::*;

pub async fn toggle(
    picker: &mut Picker,
    cx: &CommandContext,
    id: &str,
    models: &[CatalogEntry],
) -> Result<(), KernelError> {
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
    .await?;
    if pinned {
        picker.props.selected = Some(Row::Model(id.into()).key());
    }
    let model = id.split_once('/').map_or(id, |(_, model)| model);
    announce(
        cx,
        format!("{} {model}", if pinned { "unpinned" } else { "pinned" }),
    )
    .await
}

async fn announce(cx: &CommandContext, text: String) -> Result<(), KernelError> {
    let until = (std::time::SystemTime::now() + std::time::Duration::from_secs(3))
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
