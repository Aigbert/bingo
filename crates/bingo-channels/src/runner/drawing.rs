use super::*;
use bingo_channel_adapter::drawing::Drawing;
use bingo_sdk::{Action, SessionId, render::RenderRequest};
use serde_json::Value;

const REMEMBERED: usize = 64;

impl Runner {
    fn drawing(&self, id: &str) -> Option<&(Drawing, Posted)> {
        self.drawn
            .iter()
            .find(|(drawing, _)| drawing.request.request_id == id)
    }

    fn remember(&mut self, drawing: Drawing, at: Posted) {
        if let Some(previous) = self
            .drawn
            .iter_mut()
            .find(|(previous, _)| previous.request.request_id == drawing.request.request_id)
        {
            *previous = (drawing, at);
            return;
        }
        if self.drawn.len() >= REMEMBERED {
            self.drawn.pop_front();
        }
        self.drawn.push_back((drawing, at));
    }

    pub(super) async fn draw_site(
        &mut self,
        request: RenderRequest,
        fallback: Op,
    ) -> Result<(), ChannelError> {
        if let Some((drawing, _)) = self.drawing(&request.request_id)
            && drawing.request == request
        {
            return Ok(());
        }
        let view = match self
            .host
            .render(&self.tree.root().summary.id, request.clone())
            .await
        {
            Ok(view) => view,
            Err(error) => {
                tracing::warn!(%error, "the chat site could not be drawn");
                return Box::pin(self.deliver(fallback)).await;
            }
        };
        let footer = if matches!(fallback, Op::Finalize { .. }) {
            self.adapter.view_footer(self.tree.root())
        } else {
            String::new()
        };
        let drawing = Drawing {
            session: self.tree.root().summary.id.clone(),
            cwd: self.tree.root().summary.cwd.clone().into(),
            request,
            view,
            footer,
        };
        let at = if matches!(fallback, Op::Finalize { .. }) {
            self.streaming.clone()
        } else {
            None
        };
        match self.post_drawing(&drawing, at.as_ref()).await {
            Ok(at) => {
                if matches!(fallback, Op::Finalize { .. }) {
                    self.streaming = None;
                }
                self.remember(drawing, at);
                Ok(())
            }
            Err(error) => {
                tracing::warn!(%error, "the chat refused a view; using its own drawing");
                Box::pin(self.deliver(fallback)).await
            }
        }
    }

    async fn post_drawing(
        &mut self,
        drawing: &Drawing,
        at: Option<&Posted>,
    ) -> Result<Posted, ChannelError> {
        if let Some(buttons) = self.adapter.buttons() {
            return buttons
                .draw(
                    &self.conversation,
                    self.directory.parent(&drawing.session).as_ref(),
                    at,
                    drawing,
                )
                .await;
        }
        let text = self.adapter.limits().clip(&drawing.fold()).into_owned();
        if let (Some(edit), Some(at)) = (self.adapter.edit(), at) {
            edit.finish(at, &text).await?;
            Ok(at.clone())
        } else {
            self.post(&text).await
        }
    }

    pub(super) async fn redraw(&mut self, requests: &[String]) {
        if self.adapter.edit().is_none() {
            return;
        }
        for id in requests {
            let Some((mut drawing, at)) = self.drawing(id).cloned() else {
                continue;
            };
            match self
                .host
                .render(&drawing.session, drawing.request.clone())
                .await
            {
                Ok(view) => drawing.view = view,
                Err(error) => {
                    tracing::warn!(%error, "the chat site could not be redrawn");
                    continue;
                }
            }
            match self.post_drawing(&drawing, Some(&at)).await {
                Ok(at) => {
                    self.remember(drawing, at);
                }
                Err(error) => tracing::warn!(%error, "the chat could not replace a drawn site"),
            }
        }
    }

    pub(super) async fn pressed(
        &mut self,
        principal: &str,
        session: SessionId,
        id: &str,
        action: Action,
        value: Option<Value>,
    ) {
        let allowed = self
            .drawing(id)
            .is_some_and(|(drawing, _)| drawing.session == session && drawing.permits(&action));
        if !allowed {
            if let Err(error) = self.post(bingo_sdk::view::STALE_CONTROL).await {
                tracing::warn!(%error, "the stale control notice could not be sent");
            }
            return;
        }
        let action = match value {
            Some(value) => action.with_value(value),
            None => Some(action),
        };
        let Some(action) = action else {
            if let Err(error) = self.post(bingo_sdk::view::STALE_CONTROL).await {
                tracing::warn!(%error, "the stale control notice could not be sent");
            }
            return;
        };
        let intent = IntentId::mint();
        let input = Input::action(action, incoming_origin(principal, &self.key));
        self.handle.submit(intent.clone(), input.clone());
        self.deliverer.expecting_input(intent, input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures,
        loopback::{Loopback, Record},
    };
    use bingo_sdk::testing::{HostCall, TestHost};
    use bingo_sdk::{IntentOutcome, Item, ItemBody, ItemStatus, Level};
    use serde_json::json;

    async fn runner(matched: bool, edits: bool) -> (Runner, TestHost, Arc<Loopback>) {
        let host = TestHost::with_sessions(Vec::new());
        let adapter = Arc::new(Loopback::new(crate::loopback::Config {
            edits,
            ..Default::default()
        }));
        let mut runner = Runner::open(
            &host.handle(),
            adapter.clone(),
            Conversation::direct("chat"),
            "/tmp".into(),
            Gate::default(),
            Directory::default(),
            mpsc::channel(1).1,
        )
        .await
        .unwrap();
        if matched {
            let mut state = runner.tree.root().clone();
            state.config.kernel["render"] = json!({"AssistantMessage":[{"surface":"chat"}]});
            runner.tree = TreeState::new(state);
        }
        (runner, host, adapter)
    }
    async fn answer(runner: &mut Runner) {
        for mut frame in [
            fixtures::turn_started(1),
            fixtures::said(2, "answer", "hello"),
            fixtures::turn_completed(3),
        ] {
            frame.session = runner.tree.root().summary.id.clone();
            runner.saw(&frame).await;
        }
    }
    fn renders(host: &TestHost) -> usize {
        host.calls()
            .iter()
            .filter(|call| matches!(call, HostCall::Render { .. }))
            .count()
    }
    #[tokio::test]
    async fn unmatched_chat_never_renders_and_matched_answer_is_asked_once() {
        let (mut plain, host, _) = runner(false, true).await;
        answer(&mut plain).await;
        assert_eq!(renders(&host), 0);
        let (mut matched, host, adapter) = runner(true, true).await;
        answer(&mut matched).await;
        assert_eq!(renders(&host), 1);
        let (drawing, at) = matched.drawn.front().unwrap().clone();
        matched
            .draw_site(
                drawing.request.clone(),
                Op::Card {
                    text: "unused".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(renders(&host), 1, "an identical request is cached");
        matched.redraw(&[drawing.request.request_id]).await;
        assert_eq!(renders(&host), 2);
        assert!(
            matches!(adapter.records().last(),Some(Record::Draw {at:updated,..}) if updated == &at)
        );
    }
    /// A control's recorded notice, as its ack and its transcript item.
    async fn notice_from_a_press(runner: &mut Runner, level: Level, said: bool) {
        let intent = IntentId::mint();
        runner.deliverer.expecting_input(
            intent.clone(),
            Input::Action {
                origin: Some(bingo_sdk::Origin::surface("channels")),
                action: Action {
                    name: "fixture:ui".into(),
                    args: json!({"handler":"minted"}),
                },
            },
        );
        let item = Item {
            intent: Some(intent.clone()),
            ..fixtures::assistant("itm_receipt", "", ItemStatus::Completed)
        };
        let item = Item {
            body: ItemBody::Notice {
                level,
                code: "RECEIPT".into(),
                text: "⇄ other · fake".into(),
            },
            ..item
        };
        let mut result = json!({"item": "itm_receipt"});
        if said {
            result["message"] = json!("⇄ other · fake");
        }
        for mut frame in [
            fixtures::frame(1, Event::ItemCompleted { item }),
            fixtures::frame(
                2,
                Event::IntentAck {
                    intent,
                    outcome: IntentOutcome::Applied { result },
                },
            ),
        ] {
            frame.session = runner.tree.root().summary.id.clone();
            runner.saw(&frame).await;
        }
    }
    #[tokio::test]
    async fn a_pressed_control_s_notice_is_said_once() {
        for (level, said) in [(Level::Info, true), (Level::Warn, false)] {
            let (mut runner, _host, adapter) = runner(false, true).await;
            notice_from_a_press(&mut runner, level, said).await;
            let lines = adapter
                .records()
                .into_iter()
                .filter(|record| {
                    matches!(record, Record::Send { text, .. } if text.contains("⇄ other · fake"))
                })
                .count();
            assert_eq!(lines, 1, "{level:?}: {:?}", adapter.records());
        }
    }
    #[tokio::test]
    async fn a_chat_without_edits_does_not_reask_delivered_sites() {
        let (mut runner, host, _) = runner(true, false).await;
        answer(&mut runner).await;
        let id = runner.drawn.front().unwrap().0.request.request_id.clone();
        runner.redraw(&[id]).await;
        assert_eq!(renders(&host), 1);
    }
    #[tokio::test]
    async fn oldest_drawing_is_evicted_and_its_controls_are_stale() {
        let (mut runner, host, adapter) = runner(true, true).await;
        answer(&mut runner).await;
        let (mut first, at) = runner.drawn.front().unwrap().clone();
        let action = Action {
            name: "fixture:ui".into(),
            args: json!({"handler":"minted"}),
        };
        first.view =
            serde_json::from_value(json!({"kind":"Button","children":["Press"],"onPress":action}))
                .unwrap();
        runner.remember(first.clone(), at.clone());
        assert!(
            runner
                .drawing(&first.request.request_id)
                .unwrap()
                .0
                .permits(&action)
        );
        for i in 1..=REMEMBERED {
            let mut request = first.request.clone();
            request.request_id = format!("answer-{i}");
            runner
                .draw_site(
                    request,
                    Op::Card {
                        text: "unused".into(),
                    },
                )
                .await
                .unwrap();
            if i == REMEMBERED - 1 {
                runner.remember(first.clone(), at.clone());
                assert_eq!(
                    runner.drawn.len(),
                    REMEMBERED,
                    "replacements keep their original age"
                );
            }
        }
        assert_eq!(runner.drawn.len(), REMEMBERED);
        assert!(runner.drawing(&first.request.request_id).is_none());
        assert!(runner.drawing("answer-1").is_some());
        assert!(runner.drawing(&format!("answer-{REMEMBERED}")).is_some());
        runner
            .pressed(
                "member",
                first.session,
                &first.request.request_id,
                action,
                None,
            )
            .await;
        assert!(
            matches!(adapter.records().last(), Some(Record::Send { text, .. })
            if text == bingo_sdk::view::STALE_CONTROL)
        );
        let before = renders(&host);
        runner.redraw(&[first.request.request_id]).await;
        assert_eq!(renders(&host), before);
    }
    #[tokio::test]
    async fn a_drawn_control_keeps_the_member_who_pressed_it() {
        let (mut runner, _, _) = runner(true, true).await;
        let submitted = Arc::new(crate::host::tests::TestSession::default());
        runner.handle = bingo_sdk::SessionHandle(submitted.clone());
        answer(&mut runner).await;
        let (mut drawing, at) = runner.drawn.front().unwrap().clone();
        let action = Action {
            name: "permission".into(),
            args: json!("bypassPermissions"),
        };
        drawing.view =
            serde_json::from_value(json!({"kind":"Button","children":["Press"],"onPress":action}))
                .unwrap();
        runner.remember(drawing.clone(), at);
        runner
            .heard(Incoming::Action {
                conversation: runner.conversation.clone(),
                principal: "member".into(),
                session: drawing.session,
                request_id: drawing.request.request_id,
                action,
                value: None,
            })
            .await;
        assert_eq!(
            submitted.origins(),
            [incoming_origin("member", &runner.key)]
        );
    }
}
