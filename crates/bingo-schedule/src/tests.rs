//! What the tests in this crate share: a home on disk with a store over it,
//! the context the sdk hands a tool, and the host that writes down what it
//! was told.
//!
//! A tool over this store writes files, rings a bell and publishes what
//! stands; one that asked the kernel for anything else fails loudly here.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    Answer, AnswerSpec, CancellationToken, CommandContext, Delivery, Env, HostApi, HostHandle,
    Input, InteractionKind, ItemBody, ItemId, KernelError, Prompter, SessionId, ToolContext,
    ToolHost, ToolOutput, TurnId, testing::NoHost,
};
use serde_json::Value;

use crate::schedules::Schedules;
use crate::store::{Shelf, Store};

pub(crate) struct Fixture {
    home: tempfile::TempDir,
    pub(crate) schedules: Arc<Schedules>,
    pub(crate) host: Arc<Listening>,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let home = tempfile::tempdir().expect("a temp home");
        let env = Env::rooted(home.path());
        let schedules = Arc::new(Schedules::new(&env.data_dir));
        Self {
            home,
            schedules,
            host: Arc::new(Listening::default()),
        }
    }

    pub(crate) fn handle(&self) -> HostHandle {
        self.host.host.handle()
    }

    pub(crate) fn cwd(&self) -> PathBuf {
        self.home.path().to_path_buf()
    }

    pub(crate) fn shelf(&self) -> Shelf {
        self.schedules.store().load()
    }

    /// A store of its own over the same directory: what the runner is handed.
    pub(crate) fn store(&self) -> Arc<Store> {
        Arc::new(Store::new(&Env::rooted(self.home.path()).data_dir))
    }

    pub(crate) fn dir(&self) -> PathBuf {
        self.schedules.store().dir().to_path_buf()
    }

    pub(crate) fn context(&self) -> ToolContext {
        ToolContext {
            call_id: "call_test".into(),
            session: SessionId::from_raw("ses_test"),
            turn: TurnId::from_raw("trn_test"),
            item: ItemId::from_raw("itm_test"),
            cwd: self.cwd(),
            cancel: CancellationToken::new(),
            env: Arc::new(Env::rooted(self.home.path())),
            host: self.handle(),
            call: Arc::new(Silent),
        }
    }

    pub(crate) fn command(&self) -> CommandContext {
        CommandContext {
            session: SessionId::from_raw("ses_test"),
            cwd: self.cwd(),
            host: self.handle(),
        }
    }
}

/// A tool here asks nobody anything and records nothing outside its result.
struct Silent;

#[async_trait]
impl Prompter for Silent {
    async fn ask(
        &self,
        _kind: InteractionKind,
        _answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        unreachable!("a schedule tool asks nobody anything")
    }
}

#[async_trait]
impl ToolHost for Silent {
    fn progress(&self, _item: &ItemId, _tail: String) {}

    async fn record(&self, _body: ItemBody) -> Result<ItemId, KernelError> {
        unreachable!("a schedule tool records nothing of its own")
    }
}

/// The shared host plus a gate a dispatch test can hold open.
#[derive(Debug)]
pub(crate) struct Listening {
    host: bingo_sdk::testing::TestHost,
    opening: Arc<Opening>,
}

#[derive(Debug, Default)]
struct Opening {
    blocked: std::sync::atomic::AtomicBool,
    entered: tokio::sync::Notify,
    proceed: tokio::sync::Notify,
}

impl Default for Listening {
    fn default() -> Self {
        let host = bingo_sdk::testing::TestHost::default();
        let opening = Arc::new(Opening::default());
        host.on_deliver(|_, _, _, _| async { Ok(()) });
        host.on_extend(|_, _, _, _| async { Ok(()) });
        host.on_open({
            let opening = opening.clone();
            move |selector, who, options| {
                let opening = opening.clone();
                async move {
                    if opening.blocked.load(std::sync::atomic::Ordering::SeqCst) {
                        opening.entered.notify_one();
                        opening.proceed.notified().await;
                    }
                    NoHost.open(selector, who, options).await
                }
            }
        });
        Self { host, opening }
    }
}

impl Listening {
    pub(crate) fn hold_opens(&self) {
        self.opening
            .blocked
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub(crate) async fn wait_for_open(&self) {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.opening.entered.notified(),
        )
        .await
        .expect("the dispatch reaches open");
    }

    pub(crate) fn release_opens(&self) {
        self.opening.proceed.notify_one();
    }

    pub(crate) fn delivered(&self) -> Vec<(SessionId, Input, Delivery)> {
        self.host
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                bingo_sdk::testing::HostCall::Deliver {
                    to,
                    input,
                    delivery,
                    ..
                } => Some((to, input, delivery)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn extended(&self) -> Vec<(SessionId, String, String, Value)> {
        self.host
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                bingo_sdk::testing::HostCall::Extend {
                    session,
                    plugin,
                    kind,
                    payload,
                } => Some((session, plugin, kind, payload)),
                _ => None,
            })
            .collect()
    }
}

/// The text a tool answered with, as the model reads it.
pub(crate) fn text(out: &ToolOutput) -> String {
    out.parts
        .iter()
        .filter_map(bingo_sdk::ContentPart::as_text)
        .collect()
}

/// The files in the store, in name order.
pub(crate) fn files(dir: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .flatten()
        .map(|file| file.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
