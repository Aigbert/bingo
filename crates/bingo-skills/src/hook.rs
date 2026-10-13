//! Both doors into a skill publish the same row.
use std::sync::Arc;

use bingo_sdk::{Hook, HookContext, HookMatcher, HookPoint, Item, ItemBody, Phase, TurnId};

use crate::Library;

pub(crate) struct SkillRows(pub Arc<Library>);

#[async_trait::async_trait]
impl Hook for SkillRows {
    fn id(&self) -> &str {
        "skills.rows"
    }

    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Turn],
            tool: None,
        }
    }

    async fn on_turn(&self, phase: Phase, turn: &TurnId, items: &[Item], cx: &HookContext) {
        if phase != Phase::Start {
            return;
        }
        let Some(item) = items.iter().rev().find(|item| {
            item.turn.as_ref() == Some(turn) && matches!(item.body, ItemBody::User { .. })
        }) else {
            return;
        };
        let Some((name, args)) = requested(item) else {
            return;
        };
        if self
            .0
            .skills(&cx.cwd)
            .iter()
            .any(|skill| skill.name == name)
        {
            place(&cx.host, &cx.session, &item.id, name, args).await;
        }
    }
}

fn requested(item: &Item) -> Option<(&str, &str)> {
    let ItemBody::User { parts, origin } = &item.body else {
        return None;
    };
    if origin.is_context() {
        return None;
    }
    let line = parts
        .iter()
        .find_map(bingo_sdk::ContentPart::as_text)?
        .lines()
        .next()?;
    let rest = line.strip_prefix('/')?;
    Some(
        rest.split_once(char::is_whitespace)
            .map(|(name, args)| (name, args.trim()))
            .unwrap_or((rest, "")),
    )
}

pub(crate) fn row(item: &bingo_sdk::ItemId, name: &str, args: &str) -> bingo_sdk::slot::Slot {
    bingo_sdk::slot::Slot::Row {
        caption: (!args.is_empty()).then(|| args.to_string()),
        item: item.clone(),
        mark: Some('❖'),
        title: Some("Skill".into()),
        badge: Some(name.into()),
        body: None,
        actions: vec![],
    }
}

pub(crate) async fn place(
    host: &bingo_sdk::HostHandle,
    session: &bingo_sdk::SessionId,
    item: &bingo_sdk::ItemId,
    name: &str,
    args: &str,
) {
    let payload = bingo_sdk::slot::with(serde_json::json!({}), [row(item, name, args)]);
    let _ = host
        .extend(session, crate::MANIFEST.id, &format!("row:{item}"), payload)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{
        Tool,
        testing::{HostCall, TestHost},
    };

    #[tokio::test]
    async fn both_doors_place_the_same_row_and_ignore_old_prompts() {
        let tree = crate::tests::Tree::new();
        let library = Arc::new(Library::new(bingo_sdk::Env::rooted(tree.root())));
        let host = TestHost::default();
        let mut tool_cx = crate::tests::tool_context(&tree.cwd());
        tool_cx.host = host.handle();
        crate::SkillTool::new(library.clone())
            .call(
                serde_json::json!({"name":"guide", "arguments":"the wire"}),
                &tool_cx,
            )
            .await
            .unwrap();
        let event: bingo_sdk::Event = serde_json::from_str(include_str!(
            "../../bingo-sdk/fixtures/record/event/itemCompleted.json"
        ))
        .unwrap();
        let bingo_sdk::Event::ItemCompleted { mut item } = event else {
            panic!("fixture")
        };
        item.id = tool_cx.item.clone();
        item.turn = Some(tool_cx.turn.clone());
        item.body = ItemBody::User {
            parts: vec![bingo_sdk::ContentPart::text(
                "/guide the wire\n\nexpanded body",
            )],
            origin: bingo_sdk::Origin::surface("command").voiced(bingo_sdk::Voice::Quiet),
        };
        let cx = HookContext {
            item: None,
            session: tool_cx.session.clone(),
            turn: Some(tool_cx.turn.clone()),
            cwd: tree.cwd(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        let hook = SkillRows(library);
        hook.on_turn(Phase::Start, &tool_cx.turn, &[item.clone()], &cx)
            .await;
        hook.on_turn(Phase::End, &tool_cx.turn, &[item.clone()], &cx)
            .await;
        hook.on_turn(Phase::Start, &TurnId::from_raw("next"), &[item], &cx)
            .await;
        let rows: Vec<_> = host
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                HostCall::Extend { payload, .. } => Some(payload),
                _ => None,
            })
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], rows[1]);
        let slots: Vec<bingo_sdk::slot::Slot> =
            serde_json::from_value(rows[0]["slots"].clone()).unwrap();
        assert_eq!(slots, [row(&tool_cx.item, "guide", "the wire")]);
    }
}
