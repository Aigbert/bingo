use super::*;

#[async_trait]
impl HostApi for Host {
    async fn approve(&self, request: Approval) -> Result<String, KernelError> {
        approval::approve(self, request)
    }

    async fn complete(
        &self,
        session: &SessionId,
        turn: Option<&TurnId>,
        request: Completion,
    ) -> Result<Vec<ModelEvent>, KernelError> {
        completion::complete(self, session, turn, request).await
    }

    async fn sessions(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        let live: Vec<Live> = self.lock().values().cloned().collect();
        let mut out = Vec::new();
        for session in live {
            if filter
                .cwd
                .as_ref()
                .is_some_and(|cwd| cwd != &session.spec.cwd)
            {
                continue;
            }
            if filter
                .parent
                .as_ref()
                .is_some_and(|p| session.spec.parent.as_ref().map(|link| &link.session) != Some(p))
            {
                continue;
            }
            if let Ok(summary) = session.mailbox.summary().await {
                out.push(summary);
            }
        }
        if let Some(store) = &self.registry.store {
            let live_ids: Vec<SessionId> = out.iter().map(|s| s.id.clone()).collect();
            out.extend(
                store
                    .list(&filter)
                    .await?
                    .into_iter()
                    .filter(|s| !live_ids.contains(&s.id)),
            );
        }
        out.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        if let Some(limit) = filter.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    async fn open(
        &self,
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        let mailbox = match selector {
            SessionSelector::Create { spec } => self.create(spec).await?,
            other => self.mailbox_for(other).await?,
        };
        if options.children {
            return tree::attach(self.weak.clone(), &self.gateway, mailbox, who, options).await;
        }
        let (snapshot, history, events) =
            mailbox.attach_bounded(options.max_snapshot_bytes).await?;
        Ok(Attachment {
            session: mailbox.id().clone(),
            snapshot,
            history,
            events,
            handle: mailbox.port(who),
        })
    }

    async fn close(&self, session: &SessionId, _reason: CloseReason) -> Result<(), KernelError> {
        // Detaching is dropping the attachment; the session keeps running.
        self.live(session).map(|_| ())
    }

    async fn read(&self, session: &SessionId) -> Result<SessionState, KernelError> {
        inspection::read(self, session).await
    }

    async fn summary(&self, session: &SessionId) -> Result<SessionSummary, KernelError> {
        inspection::summary(self, session).await
    }

    async fn check_delete(&self, session: &SessionId) -> Result<(), KernelError> {
        inspection::check_delete(self, session).await
    }

    async fn open_elsewhere(&self, session: &SessionId) -> Result<bool, KernelError> {
        inspection::open_elsewhere(self, session).await
    }

    async fn fork(
        &self,
        session: &SessionId,
        at: Option<TurnId>,
    ) -> Result<SessionId, KernelError> {
        fork::fork_session(self, session, at).await
    }

    /// A session and everything under it, children first (ADR-0010 §6).
    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        deletion::delete(self, session).await
    }

    async fn deliver(
        &self,
        to: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        let mailbox = self.mailbox_of(to).await?;
        let input = catalogue::Send {
            to: to.clone(),
            input,
            delivery,
        };
        let report = crate::chain::Report::default();
        let result = crate::chain::json::dispatch(
            catalogue::SESSION_SEND,
            &self.hook_set().links().await,
            &self.catalogue_context(Some(to)),
            input,
            Next::new(|input: catalogue::Send| {
                Box::pin(async move {
                    self.mailbox_of(&input.to)
                        .await?
                        .deliver(intent, input.input, input.delivery);
                    Ok(())
                })
            }),
            report.clone(),
        )
        .await;
        for notice in report.take() {
            let _ = mailbox.record(notice).await;
        }
        result
    }

    async fn extend(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.mailbox_of(session)
            .await?
            .extend(plugin.to_string(), kind.to_string(), payload);
        Ok(())
    }

    async fn render(
        &self,
        session: &SessionId,
        request: bingo_sdk::render::RenderRequest,
    ) -> Result<View, KernelError> {
        render::render(self, session, request).await
    }
    async fn invalidate(
        &self,
        session: &SessionId,
        requests: Vec<String>,
    ) -> Result<(), KernelError> {
        render::invalidate(self, session, requests).await
    }

    async fn signal(
        &self,
        session: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        self.mailbox_of(session)
            .await?
            .signal(plugin.to_string(), kind.to_string(), payload);
        Ok(())
    }

    /// Reopened first, like `extend`: a session that is only stored still has
    /// a journal to take a turn back out of.
    async fn rewind(&self, session: &SessionId, to_turn: &TurnId) -> Result<u32, KernelError> {
        self.mailbox_of(session)
            .await?
            .rewind(to_turn.clone())
            .await
    }

    /// Transfer live signal ownership without changing the payload.
    async fn transfer_signals(
        &self,
        session: &SessionId,
        plugin: &str,
        from: &ItemId,
        to: &ItemId,
    ) -> Result<(), KernelError> {
        self.mailbox_of(session)
            .await?
            .transfer_signals(plugin.into(), from.clone(), to.clone())
            .await
    }

    /// The session must be live and running a turn: one that is only stored
    /// is answering nobody, and reopening it would open a session with no
    /// turn to serve the call anyway (ADR-0036 §2).
    async fn invoke(
        &self,
        session: &SessionId,
        call: ToolCall,
    ) -> Result<ToolOutcome, KernelError> {
        self.live(session)?.mailbox.invoke(call).await
    }

    async fn ask(
        &self,
        session: &SessionId,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        ask::put(self, session, kind, answers).await
    }

    /// The session must be live: a queue is what an actor holds, and a
    /// session that is only stored has none to take a line out of.
    async fn withdraw(
        &self,
        session: &SessionId,
        intent: &IntentId,
        who: ClientIdentity,
    ) -> Result<Input, KernelError> {
        self.live(session)?
            .mailbox
            .withdraw(intent.clone(), who.surface)
            .await
    }

    /// Reconfigure the next live turn through its spec (ADR-0008, ADR-0047).
    /// A stored title uses a brief store claim instead of starting an actor.
    async fn reconfigure(
        &self,
        session: &SessionId,
        change: SessionChange,
    ) -> Result<(), KernelError> {
        if let SessionChange::Title(title) = &change {
            let _guard = self.reopening.lock().await;
            if self.live(session).is_err() {
                let title = crate::spec::titled(title);
                return inspection::store(self)?
                    .rename(session, title.as_deref())
                    .await;
            }
        }
        let change = crate::config::change(self, session, change).await?;
        let changing = self.live(session)?.changing;
        let _guard = changing.lock().await;
        let live = self.live(session)?;
        if !matches!(change, SessionChange::Key(_)) && live.spec.driver == Driver::Log {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "a log session has no model",
            ));
        }
        if let SessionChange::Key(key) = &change {
            self.check_key_free_except(key.as_deref(), Some(session))?;
        }
        let mut spec = live.spec.clone();
        apply(change, &mut spec);
        let choice = self.model_for(&spec).await?;
        let summary = live.mailbox.summary().await?;
        let config = Arc::new(self.turn_config(&spec, &summary, choice, &live.mailbox));
        if let Some(entry) = self.lock().get_mut(session) {
            entry.spec = spec;
        }
        live.mailbox.reconfigure(config);
        Ok(())
    }

    fn catalog_changed(&self, kind: CatalogKind) {
        let _ = self.gateway.send(GatewayEvent::CatalogChanged { kind });
    }

    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        interception::catalogue_answer(self, kind, &self.catalogue_context(None)).await
    }

    async fn catalog_for(
        &self,
        session: &SessionId,
        kind: CatalogKind,
    ) -> Result<Catalog, KernelError> {
        let live = self.live(session)?;
        let cx = HookContext {
            session: session.clone(),
            cwd: live.spec.cwd,
            ..self.catalogue_context(None)
        };
        interception::catalogue_answer(self, kind, &cx).await
    }

    /// Every session that is open right now, and no other: a line that belongs
    /// to nobody in particular must not wake a session that had been put away
    /// just to hear it. Nobody open is refused, so the caller keeps the line.
    async fn notice(&self, level: Level, code: &str, text: &str) -> Result<(), KernelError> {
        let open: Vec<Live> = self.lock().values().cloned().collect();
        if open.is_empty() {
            return Err(KernelError::new(
                ErrorCode::SessionNotFound,
                "no session is open to say it to",
            ));
        }
        for session in open {
            let body = ItemBody::Notice {
                level,
                code: code.to_string(),
                text: text.to_string(),
            };
            if let Err(error) = session.mailbox.record(body).await {
                tracing::debug!(%error, code, "a notice reached no transcript");
            }
        }
        Ok(())
    }

    fn gateway_events(&self) -> GatewayStream {
        let rx = self.gateway.subscribe();
        Box::pin(futures::stream::unfold(rx, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(event) => return Some((event, rx)),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }))
    }

    fn service_any(&self, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        if key == command::COMMAND_CATALOG.name() {
            let reader: Arc<dyn command::CommandCatalog> =
                Arc::new(catalog::Scoped(self.weak.clone()));
            return Some(Arc::new(reader));
        }
        if key == APPROVALS.name() {
            let authority: Arc<dyn ApprovalReader> = self.approvals.clone();
            return Some(Arc::new(authority));
        }
        model_switch::service(key).or_else(|| self.registry.services.value(key))
    }

    fn service_wire(&self, key: &str) -> Option<Arc<dyn WireService>> {
        let owner = self.registry.services.wire(key)?;
        match self.registry.services.face(key) {
            Some(face) => Some(Arc::new(crate::chain::service::Wire {
                host: self.weak.clone(),
                owner,
                face,
            })),
            None => Some(owner),
        }
    }

    fn open_service(&self, key: &str, wire: Arc<dyn WireService>) -> Result<(), KernelError> {
        self.registry
            .services
            .open(key, wire)
            .map_err(|why| KernelError::new(ErrorCode::InvalidInput, why))
    }
}
