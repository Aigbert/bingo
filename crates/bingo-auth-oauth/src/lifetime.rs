//! A resolved endpoint stops using credentials when its address is retired.
use bingo_sdk::ProviderError;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub struct Lifetime(Arc<Mutex<Option<String>>>);

impl Lifetime {
    pub fn retire(&self, reason: &str) {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(reason.into());
    }

    pub fn hint(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn check(&self) -> Result<(), ProviderError> {
        match self.hint() {
            Some(message) => Err(ProviderError::Auth { message }),
            None => Ok(()),
        }
    }
}

pub fn origin(address: &str) -> Option<String> {
    let url = reqwest::Url::parse(address).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    Some(format!(
        "{}://{}:{}",
        url.scheme(),
        url.host()?,
        url.port_or_known_default()?
    ))
}
