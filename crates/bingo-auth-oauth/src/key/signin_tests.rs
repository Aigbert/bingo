use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct DestinationPrompt(AtomicUsize);
#[async_trait::async_trait]
impl Prompter for DestinationPrompt {
    async fn ask(
        &self,
        kind: InteractionKind,
        _: Vec<AnswerSpec>,
    ) -> Result<Answer, bingo_sdk::KernelError> {
        match self.0.fetch_add(1, Ordering::SeqCst) {
            0 => {
                let InteractionKind::Confirm { detail, .. } = kind else {
                    panic!("destination must precede paste: {kind:?}")
                };
                assert!(detail.contains("https://override.example:443"));
                assert!(detail.contains("/repo/.bingo/settings.json"));
                assert!(!detail.contains("SECRET"));
                Ok(Answer::Confirm)
            }
            1 => {
                assert!(matches!(
                    kind,
                    InteractionKind::Login {
                        flow: LoginFlow::Paste,
                        ..
                    }
                ));
                Ok(Answer::Text {
                    text: "dummy-key".into(),
                })
            }
            _ => panic!("unexpected prompt"),
        }
    }
}

#[tokio::test]
async fn repository_destination_is_shown_before_pasting() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        CredentialStore::new(dir.path().into())
            .owned_by("family")
            .at_origin(
                "https://user:SECRET@override.example/path",
                "/repo/.bingo/settings.json",
            )
            .with_repository_origin(true),
    );
    let key = ApiKey::new(
        "work",
        Places {
            variable: None,
            setting: "family.instances.work.apiKey".into(),
            file: None,
        },
        store.clone(),
        None,
    )
    .at_origin("family", "https://override.example/other", "user.toml");
    let prompt = Arc::new(DestinationPrompt(AtomicUsize::new(0)));
    key.login(prompt.clone(), None).await.unwrap();
    assert_eq!(prompt.0.load(Ordering::SeqCst), 2);
    assert_eq!(
        store.metadata("work").unwrap().unwrap().origin.as_deref(),
        Some("https://override.example:443")
    );
}
