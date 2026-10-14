//! Which provider and which model a session runs on (ADR-0004): the list
//! every reader of it reads, the one a person named or the build picks, the
//! model id, and the four owners of that model's facts resolved into one.

use std::sync::Arc;

use bingo_sdk::*;

use super::{Host, auth};
use crate::models::{self, ModelCatalog};
use crate::turn::{Endpoint, ModelChoice, ProviderSet};

impl Host {
    /// Every provider this host can choose from, resolved at the one point
    /// they are resolved (ADR-0030 §2): the registered ones and whatever the
    /// sources answer with now. `provider`, the catalogue and `/model`'s
    /// reading of `<provider>/<model>` all read this and nothing else.
    pub async fn providers(&self) -> Vec<Arc<dyn Provider>> {
        let providers = ProviderSet {
            fixed: self.registry.providers.clone(),
            sources: self.registry.sources.providers.clone(),
        }
        .gather()
        .await;
        self.registry.provider_state.instances.gather(providers)
    }

    /// The provider `id` names; with none, the settings' provider; with
    /// neither, the first that can actually answer.
    pub async fn provider(&self, id: Option<&str>) -> Result<Arc<dyn Provider>, KernelError> {
        let providers = self.providers().await;
        let named = id.map(str::to_string).or_else(|| {
            self.settings
                .kernel
                .provider
                .clone()
                .filter(|name| !self.registry.provider_state.instances.removed(name))
        });
        match named {
            Some(wanted) => by_id(&providers, &wanted),
            None => default_provider(&providers),
        }
    }

    pub(super) async fn model(
        &self,
        provider: &dyn Provider,
        wanted: Option<&str>,
    ) -> Result<String, KernelError> {
        if let Some(model) = wanted
            .map(str::to_string)
            .or_else(|| self.settings.kernel.model.clone())
        {
            return Ok(model);
        }
        let models = provider
            .models()
            .await
            .map_err(|e| KernelError::new(ErrorCode::ProviderUnavailable, e.to_string()))?;
        models.first().map(|m| m.id.clone()).ok_or_else(|| {
            KernelError::new(
                ErrorCode::InvalidInput,
                format!("no model configured for provider {}", provider.id()),
            )
        })
    }

    /// The model a spec runs on: none for a `Log` session (ADR-0011 §1),
    /// which resolves no provider and calls none.
    pub(super) async fn model_for(
        &self,
        spec: &SessionSpec,
    ) -> Result<Option<ModelChoice>, KernelError> {
        match spec.driver {
            Driver::Model => Ok(Some(self.choose_model(spec).await?)),
            Driver::Log => Ok(None),
        }
    }

    /// The spec holds the level too (ADR-0047 §1), resolved at open. Model
    /// metadata never vetoes it: only `off` omits the reasoning parameter.
    pub(super) async fn choose_model(
        &self,
        spec: &SessionSpec,
    ) -> Result<ModelChoice, KernelError> {
        let named = spec.provider.as_ref().or(self
            .settings
            .kernel
            .provider
            .as_ref()
            .filter(|name| !self.registry.provider_state.instances.removed(name)));
        let agent = self
            .registry
            .agents
            .iter()
            .find(|a| named.is_some_and(|n| n == a.id()));
        let agent = match agent {
            Some(agent) => Some(agent),
            None if named.is_none() && self.providers().await.is_empty() => {
                self.registry.agents.first()
            }
            None => None,
        };
        if let Some(agent) = agent {
            return self.choose_agent(agent.clone(), spec).await;
        }
        let provider = self.provider(spec.provider.as_deref()).await?;
        auth::check(provider.as_ref())?;
        let model = self.model(provider.as_ref(), spec.model.as_deref()).await?;
        let capabilities = self.resolve_model(provider.as_ref(), &model);
        Ok(ModelChoice {
            max_tokens: models::max_tokens(&capabilities, self.settings.kernel.max_tokens),
            reasoning: spec.thinking.flatten(),
            learned: self.learned.clone(),
            endpoint: Endpoint::Provider(provider),
            id: model,
            capabilities,
        })
    }

    async fn choose_agent(
        &self,
        agent: Arc<dyn Agent>,
        spec: &SessionSpec,
    ) -> Result<ModelChoice, KernelError> {
        let id = match spec
            .model
            .clone()
            .or_else(|| self.settings.kernel.model.clone())
        {
            Some(id) => id,
            None => agent
                .models()
                .await
                .map_err(|e| KernelError::new(ErrorCode::ProviderUnavailable, e.to_string()))?
                .first()
                .map(|m| m.id.clone())
                .ok_or_else(|| {
                    KernelError::new(ErrorCode::InvalidInput, "the agent named no models")
                })?,
        };
        let key = models::declared::key(agent.id(), &id);
        let capabilities = models::resolve(
            self.settings.kernel.models.get(&key),
            self.learned.window(agent.id(), &id),
            None,
            EndpointCapabilities {
                images: true,
                ..Default::default()
            },
        );
        Ok(ModelChoice {
            endpoint: Endpoint::Agent(agent),
            id,
            max_tokens: models::max_tokens(&capabilities, None),
            capabilities,
            reasoning: spec.thinking.flatten(),
            learned: self.learned.clone(),
        })
    }

    /// The four owners of a model's facts, read once per session (ADR-0004).
    pub(super) fn resolve_model(&self, provider: &dyn Provider, model: &str) -> ModelCapabilities {
        let key = models::declared::key(provider.id(), model);
        models::resolve(
            self.settings.kernel.models.get(&key),
            self.learned.window(provider.id(), model),
            ModelCatalog::embedded().facts_for(provider.family(), provider.id(), model),
            provider.endpoint(model),
        )
    }
}

/// The provider registered under this id.
fn by_id(providers: &[Arc<dyn Provider>], wanted: &str) -> Result<Arc<dyn Provider>, KernelError> {
    providers
        .iter()
        .find(|p| p.id() == wanted)
        .cloned()
        .ok_or_else(|| {
            let known: Vec<&str> = providers.iter().map(|p| p.id()).collect();
            KernelError::new(
                ErrorCode::ProviderUnavailable,
                format!(
                    "No provider called `{wanted}`. Registered: {}.",
                    known.join(", ")
                ),
            )
        })
}

/// Nobody named one, so the build picks: the first in registration order
/// whose credentials are in place. Registration order alone would hand a
/// person the provider they have never signed in to while another is ready
/// and waiting, and [`auth::check`] would then refuse the run outright.
fn default_provider(providers: &[Arc<dyn Provider>]) -> Result<Arc<dyn Provider>, KernelError> {
    if providers.is_empty() {
        return Err(KernelError::new(
            ErrorCode::ProviderUnavailable,
            "No model provider is registered in this build.",
        ));
    }
    providers
        .iter()
        .find(|p| auth::signed_in(p.as_ref()))
        .cloned()
        .ok_or_else(|| auth::nobody_signed_in(providers))
}
