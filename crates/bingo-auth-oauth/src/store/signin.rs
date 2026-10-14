//! Show repository-controlled destinations before accepting a credential.
use super::CredentialStore;
use crate::AuthError;
use bingo_sdk::{Answer, AnswerSpec, InteractionKind, Prompter};

impl CredentialStore {
    pub async fn confirm_sign_in(
        &self,
        name: &str,
        prompter: &dyn Prompter,
    ) -> Result<(), AuthError> {
        self.validate_address()?;
        let destinations: Vec<_> = [&self.binding, &self.resource]
            .into_iter()
            .filter_map(|binding| binding.as_ref().ok().and_then(Option::as_ref))
            .filter(|binding| binding.repository)
            .map(|binding| format!("{} (from {})", binding.origin, binding.file))
            .collect();
        if destinations.is_empty() {
            return Ok(());
        }
        let answer = prompter.ask(InteractionKind::Confirm {
            title: format!("Sign in to {name}"),
            detail: format!("Repository settings direct this sign-in to {}. The new credential will be bound to this address. Continue?", destinations.join(" and ")),
        }, vec![AnswerSpec::Confirm, AnswerSpec::Cancel]).await;
        match answer {
            Ok(Answer::Confirm) => Ok(()),
            _ => Err(AuthError::Cancelled),
        }
    }
}
