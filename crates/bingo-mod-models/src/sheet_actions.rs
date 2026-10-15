use crate::{guess::Guess, state::Picker};
use bingo_sdk::*;
pub fn navigate(selected: &mut usize, key: &str, count: usize) {
    *selected = (*selected).min(count.saturating_sub(1));
    match key {
        "up" => *selected = selected.saturating_sub(1),
        "down" => *selected = (*selected + 1).min(count.saturating_sub(1)),
        _ => {}
    }
}
pub async fn choose_id(
    picker: &Picker,
    id: &str,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    let model = cx
        .host
        .catalog(CatalogKind::Models)
        .await?
        .entries
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| crate::error("model is no longer listed"))?;
    crate::keys::choose(picker, &model, cx).await
}
pub async fn signin_key(
    provider: &str,
    selected: &mut usize,
    pending: &Option<String>,
    key: &str,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    navigate(selected, key, 3);
    if key != "enter" {
        return Ok(false);
    }
    let method = ["browser", "device", "paste"][*selected];
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "login",
        &format!("{provider} {method}"),
        None,
    )
    .await?;
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "models",
        &format!("refresh {provider}"),
        None,
    )
    .await?;
    if let Some(id) = pending {
        return choose_id(picker, id, cx).await;
    }
    picker.sheet = Some(crate::sheet::Sheet::Manage {
        provider: provider.into(),
        selected: 0,
        pending: None,
        confirm: false,
    });
    Ok(false)
}
pub async fn add_key(
    guess: &mut Guess,
    selected: &mut usize,
    key: &str,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<bool, KernelError> {
    navigate(selected, key, 5);
    if key != "enter" {
        return Ok(false);
    }
    match *selected {
        0 => {
            if let Some(name) = crate::bridge::paste(&cx.host, &cx.session, "provider name").await?
            {
                if !bingo_sdk::provider::valid_provider_name(&name) {
                    return Err(crate::error(
                        "Use letters, digits, hyphens or underscores for the name.",
                    ));
                }
                guess.name = name;
            }
        }
        1 => {
            guess.shape = if guess.shape == "openai" {
                "anthropic"
            } else {
                "openai"
            }
            .into()
        }
        2 => {
            if let Some(address) =
                crate::bridge::paste(&cx.host, &cx.session, "provider address").await?
            {
                guess.address = crate::guess::guess(&address, &[])
                    .filter(|g| g.key.is_none())
                    .ok_or_else(|| crate::error("Use an http(s) address without credentials."))?
                    .address;
            }
        }
        3 => {
            guess.key = crate::bridge::paste(&cx.host, &cx.session, "provider key")
                .await?
                .and_then(|pasted| crate::guess::key(&pasted))
        }
        _ => save_provider(guess, picker, cx).await?,
    }
    Ok(false)
}

async fn save_provider(
    guess: &mut Guess,
    picker: &mut Picker,
    cx: &CommandContext,
) -> Result<(), KernelError> {
    if guess.address.is_empty() {
        return Err(crate::error(
            "Paste the provider's address first: an sk- key does not say whose it is.",
        ));
    }
    crate::bridge::run(
        &cx.host,
        &cx.session,
        "provider",
        &format!(
            "add {} {} {}",
            guess.name,
            guess.shape,
            crate::guess::wire_address(&guess.shape, &guess.address)
        ),
        guess.key.take().map(|key| (guess.name.clone(), key)),
    )
    .await?;
    picker.props.filter = format!("{}/", guess.name);
    picker.props.selected = None;
    picker.sheet = Some(crate::sheet::Sheet::Manage {
        provider: guess.name.clone(),
        selected: 0,
        pending: None,
        confirm: false,
    });
    picker.message = Some("Provider added. Esc returns to the list; pick one.".into());
    Ok(())
}
