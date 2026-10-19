//! Hold the fake provider's text block open without a wall-clock race.
use bingo_sdk::*;
use futures::StreamExt;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Streaming(pub Arc<bingo_provider_fake::FakeProvider>);

#[async_trait::async_trait]
impl Provider for Streaming {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn endpoint(&self, model: &str) -> EndpointCapabilities {
        self.0.endpoint(model)
    }

    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        self.0.models().await
    }

    async fn count_tokens(&self, request: &ModelRequest) -> Result<u64, ProviderError> {
        self.0.count_tokens(request).await
    }

    async fn stream(
        &self,
        request: ModelRequest,
        cancel: CancellationToken,
    ) -> Result<ModelStream, ProviderError> {
        let stream = self.0.stream(request, cancel).await?;
        Ok(Box::pin(stream.then(move |event| async move {
            if matches!(event, Ok(ModelEvent::TextEnd { .. })) {
                futures::future::pending::<()>().await;
            }
            event
        })))
    }
}

#[async_trait::async_trait]
impl Plugin for Streaming {
    fn manifest(&self) -> &'static PluginManifest {
        &PluginManifest {
            id: "test.streaming",
            version: "0",
            uses: &[],
            provides: &["provider:fake"],
            requires: &[],
            config: None,
        }
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Provider(Arc::new(self.clone())));
        Ok(())
    }
}
