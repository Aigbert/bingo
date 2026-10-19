use bingo_sdk::*;
/// A plain word from the sheet: the reason a choice did not happen.
pub(crate) async fn say(cx: &CommandContext, text: String) -> Result<(), KernelError> {
    announce(cx, vec![text.into()], Tone::Neutral).await
}

pub(crate) async fn announce(
    cx: &CommandContext,
    parts: Vec<view::TextChild>,
    tone: Tone,
) -> Result<(), KernelError> {
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
                    parts,
                    tone,
                    until,
                    alert: false,
                }],
            ),
        )
        .await
}
