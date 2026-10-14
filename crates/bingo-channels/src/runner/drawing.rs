use super::*;
use bingo_channel_adapter::drawing::Drawing;
use bingo_sdk::{Action, SessionId, render::RenderRequest};
use serde_json::Value;

impl Runner {
    pub(super) async fn draw_site(
        &mut self,
        request: RenderRequest,
        fallback: Op,
    ) -> Result<(), ChannelError> {
        if let Some((drawing, _)) = self.drawn.get(&request.request_id)
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
                self.drawn
                    .insert(drawing.request.request_id.clone(), (drawing, at));
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
            let Some((mut drawing, at)) = self.drawn.get(id).cloned() else {
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
                    self.drawn.insert(id.clone(), (drawing, at));
                }
                Err(error) => tracing::warn!(%error, "the chat could not replace a drawn site"),
            }
        }
    }

    pub(super) async fn pressed(
        &mut self,
        session: SessionId,
        id: &str,
        mut action: Action,
        value: Option<Value>,
    ) {
        let allowed = self
            .drawn
            .get(id)
            .is_some_and(|(drawing, _)| drawing.session == session && drawing.permits(&action));
        if !allowed {
            if let Err(error) = self
                .post("This control belongs to an earlier drawing; try the current one.")
                .await
            {
                tracing::warn!(%error, "the stale control notice could not be sent");
            }
            return;
        }
        if let Some(value) = value
            && action.args.is_object()
        {
            action.args["value"] = value;
        }
        let intent = IntentId::mint();
        let input = Input::Action { action };
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
        let (drawing, at) = matched.drawn.values().next().unwrap().clone();
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
    #[tokio::test]
    async fn a_chat_without_edits_does_not_reask_delivered_sites() {
        let (mut runner, host, _) = runner(true, false).await;
        answer(&mut runner).await;
        let id = runner.drawn.keys().next().unwrap().clone();
        runner.redraw(&[id]).await;
        assert_eq!(renders(&host), 1);
    }
}
