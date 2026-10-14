use super::*;

impl Run {
    pub(super) fn refocus(&mut self) {
        let now = self.detection.now().wall;
        let open_ids: std::collections::HashSet<_> = self
            .session
            .tree
            .sessions()
            .flat_map(|state| state.interactions.iter().map(|open| open.id.clone()))
            .collect();
        self.received_asks.retain(|id, _| open_ids.contains(id));
        for id in open_ids {
            self.received_asks.entry(id).or_insert(now);
        }

        let open = self.session.tree.open_interaction().map(|(_, open)| open);
        let changed = self.ui.dialog.current() != open.map(|open| &open.id);
        self.ui.dialog.focus_on(open);
        if changed && let Some(open) = open {
            self.ui.dialog.aside = crate::ask::aside::recent(
                self.received_asks.get(&open.id).copied().unwrap_or(now),
                self.ui.last_typed,
            );
        }
    }

    pub(super) fn stop_interaction(
        &mut self,
        session: SessionId,
        interaction: bingo_sdk::InteractionId,
        answer: bingo_sdk::Answer,
    ) {
        let deny = self.mint(None);
        let stop = self.mint(None);
        if let Some(handle) = self.session.handles.get(&session) {
            handle.answer(deny, interaction, answer, bingo_sdk::Activation::Keyboard);
            handle.interrupt(stop, InterruptScope::Head);
            return;
        }
        let host = self.host.clone();
        self.spawn(async move {
            let attachment = host
                .open(
                    SessionSelector::ById {
                        id: session.clone(),
                    },
                    identity(),
                    OpenOptions::default(),
                )
                .await?;
            attachment
                .handle
                .answer(deny, interaction, answer, bingo_sdk::Activation::Keyboard);
            attachment.handle.interrupt(stop, InterruptScope::Head);
            Ok(Reply::Handle(session, attachment.handle))
        });
    }

    /// An answer goes through the root: its handle knows which session asked.
    pub(super) fn answer(
        &mut self,
        interaction: bingo_sdk::InteractionId,
        answer: bingo_sdk::Answer,
        activation: bingo_sdk::Activation,
    ) {
        let Some(handle) = self.session.root() else {
            return self.not_yet();
        };
        let intent = self.mint(None);
        handle.answer(intent, interaction, answer, activation);
    }
}
