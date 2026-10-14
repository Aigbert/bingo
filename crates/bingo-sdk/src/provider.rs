//! A model provider: one wire protocol, streaming, in the neutral vocabulary.

use std::sync::Arc;

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::host::Prompter;
use crate::model::{Effort, EndpointCapabilities, ModelRequest, ModelStream, ProviderError};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthStatus {
    NotApplicable,
    Ready {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<KeySource>,
    },
    /// No credentials; `hint` says where to put them, in the user's words.
    Missing {
        hint: String,
    },
    /// Credentials that stopped working; `hint` says how to renew them.
    Expired {
        hint: String,
    },
}

/// The place supplying a usable credential; never its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum KeySource {
    Store,
    Setting,
    Env,
}

/// Public configuration for one named endpoint.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<bool>,
}

/// One constructor for boot and runtime instances of a wire protocol.
pub trait ProviderShape: Send + Sync {
    fn id(&self) -> &str;
    fn settings_key(&self) -> &str;
    fn build(
        &self,
        name: &str,
        spec: &InstanceSpec,
    ) -> Result<Arc<dyn Provider>, crate::PluginError>;
    /// Whether adding this name would adopt an existing stored credential.
    fn credential_exists(&self, _name: &str) -> Result<bool, crate::PluginError> {
        Ok(false)
    }
    fn takes_key(&self) -> bool {
        false
    }
}

/// How a person wants to sign in (ADR-0012 §4); `None` is the provider's
/// default. The flow that results is what `LoginFlow` shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LoginMethod {
    /// A browser on this machine and a loopback callback.
    Browser,
    /// A code entered in a browser anywhere; polled here.
    Device,
    /// A credential minted elsewhere, pasted in.
    Paste,
}

impl LoginMethod {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "browser" => Some(Self::Browser),
            "device" => Some(Self::Device),
            "paste" => Some(Self::Paste),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// The provider id configuration refers to (`anthropic`, `openai`, `codex`, `fake`).
    fn id(&self) -> &str;

    /// The model family this provider serves — what a model catalogue files
    /// its models under. A named instance (ADR-0017) answers the shape it
    /// speaks (`openai`, `anthropic`, `codex`); the default is the id, which
    /// is where the built-ins are filed.
    fn family(&self) -> &str {
        self.id()
    }

    /// The registered constructor, if this provider supports named instances.
    fn shape(&self) -> Option<&str> {
        None
    }

    /// The configured address. Catalogue readers must strip credentials.
    fn address(&self) -> Option<&str> {
        None
    }

    fn instance(&self) -> bool {
        false
    }

    /// The origin within which this provider may reuse a credential.
    fn credential_origin(&self) -> Option<String> {
        None
    }

    /// Stop resolved sessions from reading credentials for a retired endpoint.
    fn retire(&self, _reason: &str) {}

    /// Replace endpoint options; a changed origin discards configured credentials.
    fn readdress(&self, _spec: &InstanceSpec) -> Result<Arc<dyn Provider>, crate::PluginError> {
        Err(crate::PluginError::Config(
            "this provider cannot be re-addressed".into(),
        ))
    }

    /// What this endpoint does with a request for `model`. Fails closed: a
    /// provider that does not know says `false`.
    fn endpoint(&self, model: &str) -> EndpointCapabilities;

    /// Distinct effort levels this adapter can encode for a model. The
    /// catalogue applies model-owned reasoning metadata before offering them.
    fn efforts(&self, _model: &str) -> Vec<Effort> {
        Effort::ALL.to_vec()
    }

    /// Stream one response. Non-streaming completion is this, drained.
    async fn stream(
        &self,
        request: ModelRequest,
        cancel: CancellationToken,
    ) -> Result<ModelStream, ProviderError>;

    async fn count_tokens(&self, _request: &ModelRequest) -> Result<u64, ProviderError> {
        Err(ProviderError::Unsupported {
            message: "count_tokens".into(),
        })
    }

    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }

    /// Fetch the live catalogue without substituting an offline model menu.
    async fn probe_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        self.models().await
    }

    fn auth(&self) -> AuthStatus {
        AuthStatus::NotApplicable
    }

    /// Interactive login (ADR-0012 §4): an OAuth flow shows itself through
    /// the prompter as `InteractionKind::Login` and answers with a receipt
    /// a person can read (`Signed in to codex as …`).
    async fn login(
        &self,
        _prompter: Arc<dyn Prompter>,
        _method: Option<LoginMethod>,
    ) -> Result<String, ProviderError> {
        Err(ProviderError::Unsupported {
            message: "login".into(),
        })
    }

    /// Forget the stored credential, revoking it where the issuer allows.
    async fn logout(&self) -> Result<String, ProviderError> {
        Err(ProviderError::Unsupported {
            message: "logout".into(),
        })
    }
}

#[cfg(test)]
mod runtime_contract {
    use super::*;
    use serde_json::json;

    #[test]
    fn ready_is_additive_and_names_only_the_source() {
        assert_eq!(
            serde_json::from_value::<AuthStatus>(json!({"kind":"ready"})).unwrap(),
            AuthStatus::Ready { from: None }
        );
        for (source, spelling) in [
            (KeySource::Store, "store"),
            (KeySource::Setting, "setting"),
            (KeySource::Env, "env"),
        ] {
            let status = AuthStatus::Ready { from: Some(source) };
            assert_eq!(
                serde_json::to_value(&status).unwrap(),
                json!({"kind":"ready", "from":spelling})
            );
            assert_eq!(
                serde_json::from_value::<AuthStatus>(serde_json::to_value(&status).unwrap())
                    .unwrap(),
                status
            );
        }
    }

    #[test]
    fn an_instance_spec_contains_no_credential() {
        let spec: InstanceSpec =
            serde_json::from_value(json!({"baseUrl":"http://localhost:1234", "images":false}))
                .unwrap();
        assert_eq!(
            serde_json::to_value(spec).unwrap(),
            json!({"baseUrl":"http://localhost:1234", "images":false})
        );
    }
}
