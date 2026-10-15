//! Repository adapter commands stay out of the model catalogue until approved.
use bingo_sdk::*;

pub(crate) struct WaitingAdapters(pub Vec<Waiting>);

#[async_trait::async_trait]
impl PluginSource for WaitingAdapters {
    fn id(&self) -> &str {
        "bingo.acp"
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        self.0
            .iter()
            .map(|entry| PluginStatus {
                id: format!("acp:{}", entry.name),
                version: String::new(),
                state: PluginState::Waiting {
                    changed: entry.changed,
                },
                from: "bingo.acp".into(),
                placement: None,
                grant: None,
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl Command for WaitingAdapters {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "acp".into(),
            hint: "approve a repository adapter for the next start".into(),
            args: ArgSpec::Words {
                person: vec!["approve".into()],
                alone: vec![],
                values: vec!["approve".into()],
                then: Some(Box::new(ArgSpec::Words {
                    person: vec![],
                    alone: vec![],
                    values: self.0.iter().map(|entry| entry.name.clone()).collect(),
                    then: None,
                })),
            },
            aliases: vec![],
            keys: vec![],
            hidden: false,
            instant: true,
            family: "acp".into(),
        }
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        if !render::asked() {
            return Err(KernelError::new(
                ErrorCode::PermissionDenied,
                "approval requires person input",
            ));
        }
        let name = match self.spec().person_verb(args) {
            Some(("approve", name)) => Some(name),
            _ => None,
        };
        let entry = self
            .0
            .iter()
            .find(|entry| Some(entry.name.as_str()) == name)
            .ok_or_else(|| {
                KernelError::new(
                    ErrorCode::InvalidInput,
                    "usage: /acp approve <waiting adapter>",
                )
            })?;
        cx.host
            .approve(Approval {
                root: entry.root.clone(),
                what: format!("{}:{}", entry.key, entry.name),
            })
            .await?;
        Ok(CommandOutcome::Applied {
            message: Some(format!("approved {}; adapter starts next time", entry.name)),
        })
    }
}
