use crate::state::Sheet;
use bingo_sdk::*;
pub(crate) async fn read(host: &HostHandle, session: &SessionId) -> Result<String, KernelError> {
    Ok(exchange(&host.read(session).await?))
}
pub fn exchange(state: &SessionState) -> String {
    let items = state.transcript.items();
    let Some(at) = items.iter().rposition(|item| asked(item).is_some()) else {
        return "No exchange in this history window.".into();
    };
    let ask = asked(&items[at])
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or_default();
    let answer = items[at + 1..]
        .iter()
        .filter_map(|item| match &item.body {
            ItemBody::Assistant { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{ask}\n\n{}",
        if answer.is_empty() {
            "No answer yet."
        } else {
            &answer
        }
    )
}
pub(crate) fn start(
    shared: crate::Shared,
    host: HostHandle,
    owner: SessionId,
    request: String,
    session: SessionId,
) {
    tokio::spawn(async move {
        let result = read(&host, &session)
            .await
            .unwrap_or_else(|error| error.message);
        if let Ok(mut state) = shared.lock()
            && let Some(cursor) = state.cursors.get_mut(&(owner.clone(), request.clone()))
            && matches!(&cursor.sheet, Sheet::Loading(id) if id == &session)
        {
            cursor.sheet = Sheet::Preview(result);
        }
        let _ = host.invalidate(&owner, vec![request]).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn preview_reads_without_opening_or_closing() {
        let host = testing::TestHost::default();
        host.on_read(|_| async {
            let mut state = SessionState::new(crate::tests::summary("row", None));
            let mut item: Item = serde_json::from_value(serde_json::json!({"id":"ask","round":0,"status":"completed","startedAt":"2026-10-15T00:00:00Z","body":{"kind":"user","parts":[{"type":"text","text":"last ask\nmore"}],"origin":{"surface":"tui"}}})).unwrap();
            let mut items = vec![item.clone()];
            item.id = ItemId::from_raw("answer");
            item.body = ItemBody::Assistant { text: "last answer".into() };
            items.push(item);
            state.transcript = items.into_iter().collect();
            Ok(state)
        });
        assert_eq!(
            read(&host.handle(), &SessionId::from_raw("row"))
                .await
                .unwrap(),
            "last ask\n\nlast answer"
        );
        assert!(matches!(
            host.calls().as_slice(),
            [testing::HostCall::Read { .. }]
        ));
    }
}
