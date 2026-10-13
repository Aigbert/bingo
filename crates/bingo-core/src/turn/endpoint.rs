//! The resolved id selects one driver; an agent is never a provider adapter.
use bingo_sdk::*;
use std::sync::Arc;

#[derive(Clone)]
pub enum Endpoint {
    Provider(Arc<dyn Provider>),
    Agent(Arc<dyn Agent>),
}

impl Endpoint {
    pub fn id(&self) -> &str {
        match self {
            Self::Provider(p) => p.id(),
            Self::Agent(a) => a.id(),
        }
    }
    pub async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        match self {
            Self::Provider(p) => p.models().await,
            Self::Agent(a) => a.models().await,
        }
    }
    pub fn provider(&self) -> Option<Arc<dyn Provider>> {
        match self {
            Self::Provider(p) => Some(p.clone()),
            Self::Agent(_) => None,
        }
    }
    pub fn agent(&self) -> Option<Arc<dyn Agent>> {
        match self {
            Self::Agent(a) => Some(a.clone()),
            Self::Provider(_) => None,
        }
    }
    /// An endpoint that keeps the conversation on its own side; bingo
    /// neither compacts nor measures it.
    pub fn holds_context(&self) -> bool {
        matches!(self, Self::Agent(_))
    }
    pub async fn stream(
        &self,
        request: ModelRequest,
        cancel: CancellationToken,
    ) -> Result<ModelStream, ProviderError> {
        match self {
            Self::Provider(p) => p.stream(request, cancel).await,
            Self::Agent(_) => Err(ProviderError::Unsupported {
                message: "an agent cannot answer a model side call".into(),
            }),
        }
    }
    pub async fn count_tokens(&self, request: &ModelRequest) -> Result<u64, ProviderError> {
        match self {
            Self::Provider(p) => p.count_tokens(request).await,
            Self::Agent(_) => Err(ProviderError::Unsupported {
                message: "an agent measures its own context".into(),
            }),
        }
    }
}
