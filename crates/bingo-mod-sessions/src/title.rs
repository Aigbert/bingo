use bingo_sdk::*;

pub fn title_from(ask: &str) -> Option<String> {
    let line = ask.trim().lines().next()?.trim();
    if line.is_empty() || line.starts_with(['/', '!']) {
        return None;
    }
    if line.chars().count() <= 60 {
        return Some(line.into());
    }
    let head = line.chars().take(59).collect::<String>();
    let end = head
        .rfind(char::is_whitespace)
        .filter(|at| *at > 0)
        .unwrap_or(head.len());
    Some(format!("{}…", head[..end].trim_end()))
}
/// A title only from what the person said, by the same effective voice that
/// decides a summary's first ask, so the two never disagree.
fn spoken_title(text: &str, origin: &Origin) -> Option<String> {
    (effective_voice(origin) == Voice::Said)
        .then(|| title_from(text))
        .flatten()
}
pub(crate) struct Title;
#[async_trait::async_trait]
impl Interceptor for Title {
    fn id(&self) -> &str {
        "sessions.title"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::PROMPT_SUBMIT)
    }
    async fn prompt_submit(
        &self,
        cx: &HookContext,
        input: Input,
        next: Next<'_, Input, SubmitResult>,
    ) -> Result<SubmitResult, KernelError> {
        let result = next.run(input).await?;
        if let SubmitResult::Accepted(Input::Text { text, origin, .. }) = &result
            && let Some(title) = spoken_title(text, origin)
            && let Err(error) = title_once(&cx.host, &cx.session, title).await
        {
            tracing::warn!(session = %cx.session, %error, "automatic session title failed");
        }
        Ok(result)
    }
}
async fn title_once(host: &HostHandle, id: &SessionId, title: String) -> Result<(), KernelError> {
    let summary = host.summary(id).await?;
    if summary.title.is_none() && summary.driver != Driver::Log {
        host.reconfigure(id, SessionChange::Title(title)).await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles_are_first_lines_bounded_at_words() {
        for (ask, expected) in [
            ("  fix it\nthen test", Some("fix it")),
            ("/model x", None),
            ("!ls", None),
            ("", None),
        ] {
            assert_eq!(title_from(ask).as_deref(), expected);
        }
        let title = title_from(&"long word ".repeat(20)).unwrap();
        assert!(title.chars().count() <= 60);
        assert!(title.ends_with("word…") || title.ends_with("long…"));
        assert_eq!(title_from(&"字".repeat(100)).unwrap().chars().count(), 60);
    }
    #[test]
    fn only_a_spoken_line_titles_a_session() {
        for surface in ["command", "wake", "plugin:x", "contributor:git"] {
            assert_eq!(spoken_title("fix it", &Origin::surface(surface)), None);
        }
        assert_eq!(
            spoken_title("fix it", &Origin::surface("tui")).as_deref(),
            Some("fix it")
        );
        assert_eq!(
            spoken_title("fix it", &Origin::surface("wake").voiced(Voice::Said)).as_deref(),
            Some("fix it")
        );
        assert_eq!(
            spoken_title("fix it", &Origin::surface("tui").voiced(Voice::Note)),
            None
        );
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[tokio::test]
    async fn title_once_and_never_over_a_rename() {
        let summary = Arc::new(Mutex::new(crate::tests::summary("current", None)));
        let host = testing::TestHost::default();
        let listing = summary.clone();
        host.on_summary(move |_| {
            let s = listing.lock().unwrap().clone();
            async move { Ok(s) }
        });
        let changing = summary.clone();
        host.on_reconfigure(move |_, change| {
            let s = changing.clone();
            async move {
                if let SessionChange::Title(title) = change {
                    s.lock().unwrap().title = Some(title);
                }
                Ok(())
            }
        });
        let cx = HookContext {
            session: SessionId::from_raw("current"),
            turn: None,
            item: None,
            cwd: "/work".into(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        for text in ["first ask", "second ask"] {
            Title
                .prompt_submit(
                    &cx,
                    Input::text(text, Origin::surface("tui")),
                    Next::new(|input| Box::pin(async move { Ok(SubmitResult::Accepted(input)) })),
                )
                .await
                .unwrap();
        }
        assert_eq!(summary.lock().unwrap().title.as_deref(), Some("first ask"));
        summary.lock().unwrap().title = Some("renamed".into());
        Title
            .prompt_submit(
                &cx,
                Input::text("third ask", Origin::surface("tui")),
                Next::new(|input| Box::pin(async move { Ok(SubmitResult::Accepted(input)) })),
            )
            .await
            .unwrap();
        assert_eq!(summary.lock().unwrap().title.as_deref(), Some("renamed"));
        assert_eq!(
            host.calls()
                .iter()
                .filter(|c| matches!(c, testing::HostCall::Reconfigure { .. }))
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn summary_or_rename_errors_never_reject_an_accepted_submit() {
        for summary_fails in [true, false] {
            let host = testing::TestHost::default();
            host.on_summary(move |_| async move {
                if summary_fails {
                    Err(KernelError::new(ErrorCode::Storage, "read failed"))
                } else {
                    Ok(crate::tests::summary("current", None))
                }
            });
            host.on_reconfigure(|_, _| async {
                Err(KernelError::new(ErrorCode::Storage, "write failed"))
            });
            let cx = HookContext {
                session: SessionId::from_raw("current"),
                turn: None,
                item: None,
                cwd: "/work".into(),
                provider: None,
                model: None,
                host: host.handle(),
            };
            let result = Title
                .prompt_submit(
                    &cx,
                    Input::text("keep this", Origin::surface("tui")),
                    Next::new(|input| Box::pin(async move { Ok(SubmitResult::Accepted(input)) })),
                )
                .await
                .unwrap();
            assert!(matches!(result, SubmitResult::Accepted(_)));
            assert!(
                !host
                    .calls()
                    .iter()
                    .any(|call| matches!(call, testing::HostCall::Sessions { .. }))
            );
        }
    }
}
