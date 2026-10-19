//! `/login [provider] [browser|device|paste]` and `/logout <provider>`: a
//! provider's credential, from inside a session (ADR-0012 §5). Login is not
//! instant — it takes minutes and asks through the session's own dialog, so
//! the queue waits behind it; logout is one call and runs at once.

use std::sync::Weak;

use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::json;

use crate::host::Host;

const USAGE: &str = "usage: /login [provider] [browser|device|paste]";

pub(super) struct LoginCommand {
    pub(super) host: Weak<Host>,
}

#[async_trait]
impl Command for LoginCommand {
    fn now(&self, view: &command::SessionView<'_>) -> Option<String> {
        let missing = view
            .providers
            .iter()
            .filter(|auth| {
                matches!(
                    auth,
                    AuthStatus::Missing { .. } | AuthStatus::Expired { .. }
                )
            })
            .count();
        (missing > 0).then(|| format!("{missing} signed out"))
    }

    fn spec(&self) -> CommandSpec {
        super::spec(
            "login",
            "sign in to a provider",
            ArgSpec::Catalog {
                source: "providers".into(),
            },
            false,
        )
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let host = super::host(&self.host)?;
        let (id, method) = parse(args)?;
        // Signed out mid-session, a person means the provider that refused them.
        let session = match id {
            Some(_) => None,
            None => host.session_summary(&cx.session).await?.provider,
        };
        let provider = host.provider(id.or(session.as_deref())).await?;
        let prompter = host.prompter(&cx.session)?;
        let receipt = provider
            .login(prompter, method)
            .await
            .map_err(provider_error)?;
        Ok(receipt_of("login", provider.id(), receipt))
    }
}

pub(super) struct LogoutCommand {
    pub(super) host: Weak<Host>,
}

#[async_trait]
impl Command for LogoutCommand {
    fn spec(&self) -> CommandSpec {
        super::spec(
            "logout",
            "forget a provider's key",
            ArgSpec::Catalog {
                source: "providers".into(),
            },
            true,
        )
    }

    async fn run(&self, args: &str, _cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let host = super::host(&self.host)?;
        let id = args.trim();
        if id.is_empty() {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "usage: /logout <provider>",
            ));
        }
        let provider = host.provider(Some(id)).await?;
        let receipt = provider.logout().await.map_err(provider_error)?;
        Ok(receipt_of("logout", provider.id(), receipt))
    }
}

/// `[provider] [method]`; an unknown method is refused before any flow
/// starts. A lone method word names the method, not a provider.
fn parse(args: &str) -> Result<(Option<&str>, Option<LoginMethod>), KernelError> {
    let mut words = args.split_whitespace().peekable();
    let provider = words.next_if(|word| LoginMethod::parse(word).is_none());
    let method = match words.next() {
        None => None,
        Some(word) => Some(LoginMethod::parse(word).ok_or_else(|| {
            KernelError::new(
                ErrorCode::InvalidInput,
                format!("unknown login method `{word}`; {USAGE}"),
            )
        })?),
    };
    if words.next().is_some() {
        return Err(KernelError::new(ErrorCode::InvalidInput, USAGE));
    }
    Ok((provider, method))
}

/// The receipt is recorded in the transcript, so the person sees it where
/// they typed and the model learns what changed.
fn receipt_of(name: &str, provider: &str, receipt: String) -> CommandOutcome {
    CommandOutcome::Record {
        body: Box::new(ItemBody::Action {
            name: name.into(),
            args: json!(provider),
            result: Some(json!(receipt)),
        }),
    }
}

fn provider_error(e: ProviderError) -> KernelError {
    KernelError::new(e.code(), e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_provider_and_the_method_are_both_optional() {
        assert_eq!(parse("codex").unwrap(), (Some("codex"), None));
        assert_eq!(
            parse("codex device").unwrap(),
            (Some("codex"), Some(LoginMethod::Device))
        );
        assert_eq!(
            parse("  codex   PASTE ").unwrap(),
            (Some("codex"), Some(LoginMethod::Paste))
        );
        assert_eq!(parse("").unwrap(), (None, None));
        assert_eq!(parse("device").unwrap(), (None, Some(LoginMethod::Device)));
        assert!(parse("codex sms").unwrap_err().message.contains("sms"));
        assert!(parse("codex device extra").is_err());
    }
}
