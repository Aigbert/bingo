//! Named provider management without leaving the session.
use crate::host::{Host, Refreshed, provider};
use async_trait::async_trait;
use bingo_sdk::*;
use std::sync::Weak;

pub(super) struct ProviderCommand {
    pub host: Weak<Host>,
}
const USAGE: &str =
    "usage: /provider add <name> <shape> <url> | set <name> url <url> | remove <name>";

#[async_trait]
impl Command for ProviderCommand {
    fn spec(&self) -> CommandSpec {
        super::spec(
            "provider",
            "manage named providers",
            ArgSpec::Words {
                alone: vec![],
                values: vec!["add".into(), "set".into(), "remove".into()],
                then: None,
            },
            false,
        )
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let host = super::host(&self.host)?;
        let _edit = provider::provider_edits(&host).await;
        let words: Vec<_> = args.split_whitespace().collect();
        let message = match words.as_slice() {
            ["add", name, shape, url] => add(&host, cx, name, shape, url).await?,
            ["set", name, "url", url] => {
                let result = provider::set_provider(&host, name, url).await?;
                outcome(&host, &result, "updated")
            }
            ["remove", name] => {
                provider::remove_provider(&host, name).await?;
                format!("Removed provider {name}.")
            }
            _ => return Err(KernelError::new(ErrorCode::InvalidInput, USAGE)),
        };
        Ok(CommandOutcome::Applied {
            message: Some(message),
        })
    }
}

async fn add(
    host: &std::sync::Arc<Host>,
    cx: &CommandContext,
    name: &str,
    shape: &str,
    url: &str,
) -> Result<String, KernelError> {
    let (provider, takes_key, mut result) = provider::add_provider(host, name, shape, url).await?;
    let mut login = None;
    if takes_key {
        match provider
            .login(host.prompter(&cx.session)?, Some(LoginMethod::Paste))
            .await
        {
            Ok(_) => result = provider::refresh_provider(host, name).await?,
            Err(_) => login = Some("sign-in was not completed; use /login to try again"),
        }
    }
    let mut message = outcome(host, &result, shape);
    if let Some(login) = login {
        message.push_str(&format!(" · {login}"));
    }
    Ok(message)
}

fn outcome(host: &Host, result: &Refreshed, shape: &str) -> String {
    let detail = match &result.answer {
        Ok(count) => format!(
            "{count} models · {} ms",
            provider::provider_reach(host, &result.provider)["ms"]
        ),
        Err(error) => format!("{error}; instance saved"),
    };
    format!("+ {} · {shape} · {detail}", result.provider)
}
