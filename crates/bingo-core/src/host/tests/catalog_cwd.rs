use super::*;

struct DirectoryCommands;
#[async_trait]
impl CommandSource for DirectoryCommands {
    fn id(&self) -> &str {
        "directory"
    }
    async fn commands(&self, cwd: &std::path::Path) -> Vec<Arc<dyn Command>> {
        vec![Arc::new(DirectoryCommand(cwd.to_path_buf()))]
    }
}
struct DirectoryCommand(std::path::PathBuf);
#[async_trait]
impl Command for DirectoryCommand {
    fn spec(&self) -> CommandSpec {
        let mut spec = CommandSpec::ui("directory");
        spec.name = "directory".into();
        spec.hint = self.0.display().to_string();
        spec.hidden = false;
        spec
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Handled { handled: true })
    }
}

#[tokio::test]
async fn command_catalogues_read_each_sessions_directory() {
    let home = tempfile::tempdir().unwrap();
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::Commands(Arc::new(DirectoryCommands))],
        )],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    for name in ["first", "second"] {
        let cwd = home.path().join(name);
        std::fs::create_dir(&cwd).unwrap();
        let attached = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: cwd.clone(),
                        driver: Driver::Log,
                        ..Default::default()
                    },
                },
                who(),
                OpenOptions::default(),
            )
            .await
            .unwrap();
        let catalogue = host
            .catalog_for(&attached.session, CatalogKind::Commands)
            .await
            .unwrap();
        let entry = catalogue
            .entries
            .iter()
            .find(|e| e.id == "directory")
            .unwrap();
        assert_eq!(entry.label, cwd.display().to_string());
    }
    let unknown = host
        .catalog_for(&SessionId::from_raw("missing"), CatalogKind::Commands)
        .await;
    assert!(unknown.is_err());
    host.shutdown().await;
}
