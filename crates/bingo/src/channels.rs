//! `bingo channels add <adapter>` and `bingo channels secret <adapter>`: a
//! channel configured in one sitting, and a secret rotated on its own
//! (ADR-0020 §8, user-directed 2026-09-01: the add flow asks for the app id
//! and the secret together).
//!
//! Both run before any kernel exists, like `provider add` — an adapter is
//! built at boot, so what these write is what the *next* run reads. Two files
//! are touched, each for what it is: the app id goes to the user settings
//! layer, the secret to `auth.json` (0600) and never to a file a project
//! layer commits. The typing of a secret is not echoed and the value is never
//! printed back, here or by `doctor`.

use crate::error::invalid;
use bingo_channel_adapter::{Platform, Platforms, secret::Requirement};
use bingo_sdk::{Env, ErrorCode, KernelError};
use serde_json::Value;

use crate::login::line;
use crate::provider::unechoed;

/// Ask for everything the adapter needs — the app id in the clear (it is
/// public), the secret unechoed — and write each where it belongs.
pub async fn add(env: &Env, adapter: &str, platforms: &Platforms) -> Result<String, KernelError> {
    let (platform, wanted) = signing(adapter, platforms)?;
    let prompts = platform.prompts();
    let mut slice = serde_json::Map::new();
    for prompt in prompts.settings {
        eprint!("{}", prompt.label);
        let value = line().await?;
        if value.is_empty() {
            return Err(invalid(prompt.missing));
        }
        slice.insert(prompt.key.into(), Value::String(value));
    }
    let slice = Value::Object(slice);
    platform.parse(&slice).map_err(invalid)?;
    eprint!("{}", prompts.secret);
    let secret = unechoed().await?;
    if secret.is_empty() {
        return Err(invalid(
            "nothing was pasted; neither file was touched".to_string(),
        ));
    }
    let settings = configured(env, adapter, slice)?;
    let names = prompts
        .settings
        .iter()
        .map(|prompt| prompt.name)
        .collect::<Vec<_>>()
        .join(", ");
    let auth = bingo_channels::secret::store(env, wanted.id, secret)
        .map_err(|e| KernelError::new(ErrorCode::Internal, e))?;
    Ok([
        format!(
            "{adapter} is configured: its {names} is in {}, its secret in {} \
             under `{}`, mode 0600.",
            settings.display(),
            auth.display(),
            bingo_channels::secret::credential(adapter)
        ),
        "`bingo gateway restart` (or `start`) picks it up.".into(),
    ]
    .join("\n"))
}

/// Public settings into the user layer through the same round trip as
/// `provider add`; the platform's prompts own the keys inside its slice.
fn configured(env: &Env, adapter: &str, slice: Value) -> Result<std::path::PathBuf, KernelError> {
    let path = bingo_core::settings::user_path(env);
    let mut document = crate::provider::read(&path)?;
    let layer = serde_json::Map::from_iter([(
        bingo_channels::SETTING.into(),
        serde_json::json!({adapter: slice}),
    )]);
    merge(&mut document, layer)?;
    crate::provider::write(&path, &document)?;
    Ok(path)
}

/// The flag layer's `channels.<adapter>` object into the document, replacing
/// that adapter's entry and moving nothing beside it.
fn merge(
    document: &mut serde_json::Map<String, Value>,
    layer: serde_json::Map<String, Value>,
) -> Result<(), KernelError> {
    let Some(Value::Object(named)) = layer.get(bingo_channels::SETTING) else {
        return Ok(());
    };
    let channels = document
        .entry(bingo_channels::SETTING)
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            invalid(format!(
                "`{}` in the settings is not an object",
                bingo_channels::SETTING
            ))
        })?;
    for (name, value) in named {
        channels.insert(name.clone(), value.clone());
    }
    Ok(())
}

/// Ask for the secret, refuse an adapter that signs with nothing, and write it.
pub async fn secret(
    env: &Env,
    adapter: &str,
    platforms: &Platforms,
) -> Result<String, KernelError> {
    let (platform, wanted) = signing(adapter, platforms)?;
    let variable = wanted.variable.unwrap_or_default();
    eprint!("{}", platform.prompts().secret);
    let pasted = unechoed().await?;
    if pasted.is_empty() {
        return Err(invalid(
            "nothing was pasted; the store was not touched".to_string(),
        ));
    }
    let path = bingo_channels::secret::store(env, wanted.id, pasted)
        .map_err(|e| KernelError::new(ErrorCode::Internal, e))?;
    Ok(receipt(wanted.id, variable, &path))
}

/// The adapter, if it is one that signs with anything at all.
fn signing<'a>(
    adapter: &str,
    platforms: &'a Platforms,
) -> Result<(&'a dyn Platform, Requirement), KernelError> {
    if let Some(platform) = platforms.iter().find(|platform| platform.id() == adapter)
        && let Some(wanted) = platform.secret()
    {
        return Ok((platform.as_ref(), wanted));
    }
    let names = bingo_channels::secret::signing(platforms)
        .iter()
        .map(|wanted| wanted.id)
        .collect::<Vec<_>>()
        .join(", ");
    Err(invalid(format!(
        "`{adapter}` is not a channel that signs with a secret. These are: {names}"
    )))
}

fn receipt(adapter: &str, variable: &str, path: &std::path::Path) -> String {
    [
        format!(
            "The {adapter} secret is in {} under `{}`, mode 0600.",
            path.display(),
            bingo_channels::secret::credential(adapter)
        ),
        format!(
            "{variable} still wins wherever it is exported; this is what a \
             gateway started by launchd or systemd reads, since it inherits no shell."
        ),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_adapter_that_signs_is_accepted_and_the_refusal_lists_the_ones_that_do() {
        let platforms: Platforms = vec![std::sync::Arc::new(bingo_channel_feishu::Platform)];
        assert_eq!(
            signing("feishu", &platforms).expect("feishu signs").1.id,
            "feishu"
        );
        let refused = signing("loopback", &platforms)
            .err()
            .expect("a loopback signs with nothing");
        assert!(refused.message.contains("feishu"), "{}", refused.message);
        let refused = signing("telegram", &platforms)
            .err()
            .expect("no such adapter");
        assert!(
            refused.message.contains("`telegram`"),
            "{}",
            refused.message
        );
    }

    #[test]
    fn the_receipt_names_the_file_and_the_key_and_no_secret() {
        let said = receipt(
            "feishu",
            "BINGO_FEISHU_APP_SECRET",
            std::path::Path::new("/home/me/.bingo/data/auth.json"),
        );
        assert!(said.contains("/home/me/.bingo/data/auth.json"), "{said}");
        assert!(said.contains("`channels.feishu`"), "{said}");
        assert!(said.contains("0600"), "{said}");
        assert!(
            said.contains("BINGO_FEISHU_APP_SECRET still wins"),
            "{said}"
        );
    }
}
