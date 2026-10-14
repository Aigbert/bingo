//! Runtime instance changes through registered constructors and one settings writer.
use super::{Host, Refreshed, refresh};
use crate::settings::{self, instances};
use bingo_sdk::*;
use std::sync::Arc;

pub(crate) async fn provider_edits(host: &Host) -> tokio::sync::MutexGuard<'_, ()> {
    host.registry.provider_state.instances.command.lock().await
}

pub(crate) async fn add_provider(
    host: &Arc<Host>,
    name: &str,
    shape: &str,
    url: &str,
) -> Result<(Arc<dyn Provider>, bool, Refreshed), KernelError> {
    instances::validate_name(name)?;
    let shape = provider_shape(host, shape)?;
    let spec = InstanceSpec {
        base_url: Some(url.into()),
        images: None,
    };
    let provider = {
        let _edit = host.registry.provider_state.instances.edit.lock().await;
        if host.registry.provider_state.shapes.contains_key(name)
            || host.providers().await.iter().any(|p| p.id() == name)
            || host.registry.endpoint_name_taken(name)
        {
            return Err(invalid("that provider name is already registered"));
        }
        if shape
            .credential_exists(name)
            .map_err(|_| invalid("could not read credential store"))?
        {
            return Err(invalid(
                "a stored credential already uses that name; remove it first",
            ));
        }
        let provider = shape.build(name, &spec).map_err(|_| invalid("could not build provider; use an http(s) URL without credentials, query or fragment"))?;
        instances::add(
            &settings::user_path(host.env()),
            shape.settings_key(),
            name,
            &spec,
        )?;
        install_provider(host, name, Some(provider.clone()));
        provider
    };
    let checked = if shape.takes_key() {
        Refreshed {
            provider: name.into(),
            answer: Err("paste a key before checking models".into()),
        }
    } else {
        refresh::one(host, provider.clone()).await
    };
    Ok((provider, shape.takes_key(), checked))
}

pub(crate) async fn set_provider(
    host: &Arc<Host>,
    name: &str,
    url: &str,
) -> Result<(Arc<dyn Provider>, bool, Refreshed), KernelError> {
    let (provider, needs_key) = {
        let _edit = host.registry.provider_state.instances.edit.lock().await;
        let old = instance_provider(host, name).await?;
        let shape = provider_shape(
            host,
            old.shape()
                .ok_or_else(|| invalid("provider has no registered shape"))?,
        )?;
        let spec = InstanceSpec {
            base_url: Some(url.into()),
            images: Some(old.endpoint("").images),
        };
        let provider = old.readdress(&spec).map_err(|_| invalid("could not build provider; use an http(s) URL without credentials, query or fragment"))?;
        let changed_origin = old.credential_origin() != provider.credential_origin();
        instances::change_address(
            &settings::user_path(host.env()),
            shape.settings_key(),
            name,
            url,
            changed_origin,
        )?;
        if changed_origin {
            old.retire(&format!(
                "Provider {name} changed origin; choose it again and sign in."
            ));
            old.logout().await.map_err(|_| {
                invalid("address saved, but the old credential could not be removed")
            })?;
        }
        install_provider(host, name, Some(provider.clone()));
        (provider, changed_origin)
    };
    let result = if needs_key {
        Refreshed {
            provider: name.into(),
            answer: Err("paste a key before checking models".into()),
        }
    } else {
        refresh::one(host, provider.clone()).await
    };
    Ok((provider, needs_key, result))
}

pub(crate) async fn remove_provider(host: &Host, name: &str) -> Result<String, KernelError> {
    let _edit = host.registry.provider_state.instances.edit.lock().await;
    let provider = instance_provider(host, name).await?;
    let shape = provider_shape(
        host,
        provider
            .shape()
            .ok_or_else(|| invalid("provider has no registered shape"))?,
    )?;
    // Persist first: a file that refuses a write must not silently change live choices.
    let cleared = instances::change(
        &settings::user_path(host.env()),
        shape.settings_key(),
        name,
        None,
    )?;
    install_provider(host, name, None);
    let forgotten = provider.logout().await;
    forgotten
        .map_err(|_| invalid("instance removed, but its stored credential could not be removed"))?;
    Ok(if cleared.is_empty() {
        format!("Removed provider {name}.")
    } else {
        format!(
            "Removed provider {name}; cleared {}.",
            cleared.join(" and ")
        )
    })
}

async fn instance_provider(host: &Host, name: &str) -> Result<Arc<dyn Provider>, KernelError> {
    let provider = host.provider(Some(name)).await?;
    if !provider.instance() {
        return Err(invalid("a built-in provider cannot be changed or removed"));
    }
    Ok(provider)
}

fn provider_shape(host: &Host, id: &str) -> Result<Arc<dyn ProviderShape>, KernelError> {
    host.registry
        .provider_state
        .shapes
        .get(id)
        .cloned()
        .ok_or_else(|| invalid("unknown provider shape"))
}

fn install_provider(host: &Host, name: &str, provider: Option<Arc<dyn Provider>>) {
    host.registry
        .provider_state
        .instances
        .replace(name, provider);
    host.registry.provider_state.reach.remove(name);
    host.served.remove(name);
    for kind in [CatalogKind::Providers, CatalogKind::Models] {
        let _ = host.gateway.send(GatewayEvent::CatalogChanged { kind });
    }
}

pub(crate) async fn refresh_provider(
    host: &Arc<Host>,
    name: &str,
) -> Result<Refreshed, KernelError> {
    Ok(refresh::one(host, host.provider(Some(name)).await?).await)
}

pub(crate) fn provider_reach(host: &Host, name: &str) -> serde_json::Value {
    serde_json::json!(host.registry.provider_state.reach.get(name))
}

fn invalid(message: &str) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, message)
}
