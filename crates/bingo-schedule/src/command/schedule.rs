//! `/schedule`: the store as a person reads it — one row per entry, and the
//! line that says whether anything here will fire them (ADR-0019 §7) — and
//! the person's own `add` and `confirm` (ADR-0094).

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    ArgSpec, Command, CommandContext, CommandOutcome, CommandSpec, ErrorCode, KernelError,
};
use jiff::tz::TimeZone;

use super::consent::{self, ADD, CONFIRM};
use crate::render;
use crate::schedules::Schedules;

#[derive(Debug)]
pub struct ScheduleCommand {
    schedules: Arc<Schedules>,
}

impl ScheduleCommand {
    pub fn new(schedules: Arc<Schedules>) -> Self {
        Self { schedules }
    }
}

#[async_trait]
impl Command for ScheduleCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: "schedule".into(),
            aliases: vec!["schedules".into()],
            hint: "what fires later, and whether anything will fire it".into(),
            args: ArgSpec::Words {
                alone: vec![],
                values: [ADD, CONFIRM, "pause", "resume", "run", "forget"]
                    .map(str::to_string)
                    .to_vec(),
                // Every mutation requires the person's asked line.
                person: [ADD, CONFIRM, "pause", "resume", "run", "forget"]
                    .map(str::to_string)
                    .to_vec(),
                then: Some(Box::new(ArgSpec::Free {
                    hint: "<spec> [as <mode>] -- <text> | <id>".into(),
                })),
            },
            // Reading or writing a directory of small files touches nothing
            // a turn is using.
            instant: true,
            family: "schedule".into(),
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        match bingo_sdk::command::split_verb(args) {
            ("", _) => Ok(self.listed()),
            (ADD, rest) => consent::add(&self.schedules, rest, cx),
            (CONFIRM, rest) => consent::confirm(&self.schedules, rest),
            (verb @ ("pause" | "resume" | "run" | "forget"), rest) => {
                self.mutate(verb, rest, cx).await
            }
            (other, _) => Err(KernelError::new(
                ErrorCode::InvalidInput,
                format!(
                    "unknown /schedule verb `{other}`; use add, confirm, pause, resume, run or forget"
                ),
            )),
        }
    }
}

impl ScheduleCommand {
    async fn mutate(
        &self,
        verb: &str,
        id: &str,
        cx: &CommandContext,
    ) -> Result<CommandOutcome, KernelError> {
        use bingo_services::schedule::Service;
        consent::persons_own(verb)?;
        let owner = crate::service::Owner::new(self.schedules.clone(), cx.host.clone());
        let id = id.trim().to_string();
        let message = match verb {
            "pause" => owner.pause(id).await?,
            "resume" => owner.resume(id).await?,
            "run" => owner.run(id).await?,
            "forget" => owner.forget(id).await?,
            _ => return Err(crate::service::invalid("unknown verb")),
        };
        Ok(CommandOutcome::Applied {
            message: Some(message),
        })
    }

    fn listed(&self) -> CommandOutcome {
        CommandOutcome::View {
            view: render::view(
                &self.schedules.store().load(),
                &self.schedules.holder(),
                self.schedules.trouble().as_deref(),
                &TimeZone::system(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Entry;
    use crate::entry::tests::entry;
    use crate::tests::Fixture;
    use bingo_sdk::View;

    async fn view(fixture: &Fixture) -> View {
        let outcome = ScheduleCommand::new(fixture.schedules.clone())
            .run("", &fixture.command())
            .await
            .expect("a view");
        match outcome {
            CommandOutcome::View { view } => view,
            other => panic!("/schedule answers a view, not {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_empty_store_folds_to_two_lines() {
        let fixture = Fixture::new();
        assert_eq!(
            view(&fixture).await.fold(),
            "no schedules yet\nschedules: standby — no runner holds this store; waiting to take over"
        );
    }

    #[tokio::test]
    async fn every_entry_is_a_row_and_the_holder_is_named_under_them() {
        let fixture = Fixture::new();
        for id in ["bbbb2222", "aaaa1111"] {
            fixture
                .schedules
                .store()
                .save(&Entry {
                    id: id.into(),
                    ..entry()
                })
                .expect("an entry");
        }
        let folded = view(&fixture).await.fold();
        let rows: Vec<&str> = folded.lines().collect();
        assert!(rows[0].starts_with("id · spec · next fire"), "{folded}");
        assert!(rows[1].starts_with("aaaa1111 · every 30m · "), "{folded}");
        assert!(rows[2].starts_with("bbbb2222 · every 30m · "), "{folded}");
        assert!(rows[3].starts_with("schedules: standby"), "{folded}");
    }

    #[test]
    fn the_spec_runs_now_and_its_verbs_are_the_persons() {
        let fixture = Fixture::new();
        let spec = ScheduleCommand::new(fixture.schedules.clone()).spec();
        assert_eq!(spec.name, "schedule");
        assert_eq!(spec.aliases, ["schedules"]);
        assert!(spec.instant, "reading a directory never waits for a turn");
        assert!(spec.person_only("add every 1h -- x"));
        assert!(spec.person_only("confirm abcd"));
        assert!(!spec.person_only(""));
        assert_eq!(spec.family, "schedule");
    }

    async fn ran(
        fixture: &Fixture,
        asked: bool,
        args: &str,
    ) -> Result<CommandOutcome, KernelError> {
        let command = ScheduleCommand::new(fixture.schedules.clone());
        bingo_sdk::render::under_input(asked, command.run(args, &fixture.command())).await
    }

    #[tokio::test]
    async fn the_persons_add_writes_an_entry_that_is_theirs() {
        let fixture = Fixture::new();
        ran(&fixture, true, "add every 1h as acceptEdits -- /wake")
            .await
            .expect("an asked add");
        let shelf = fixture.shelf();
        let entry = &shelf.entries[0];
        assert!(entry.asked(), "{entry:?}");
        assert_eq!(entry.text, "/wake");
        assert_eq!(entry.permission_mode.as_deref(), Some("acceptEdits"));
        assert_eq!(entry.cwd, fixture.cwd());
    }

    #[tokio::test]
    async fn an_add_or_confirm_nobody_asked_for_writes_nothing() {
        let fixture = Fixture::new();
        let refused = ran(&fixture, false, "add every 1h -- /wake").await;
        assert_eq!(
            refused.expect_err("refused").code,
            ErrorCode::PermissionDenied
        );
        assert!(fixture.shelf().is_empty());

        fixture.schedules.store().save(&entry()).expect("an entry");
        let refused = ran(&fixture, false, "confirm abcd").await;
        assert_eq!(
            refused.expect_err("refused").code,
            ErrorCode::PermissionDenied
        );
        assert!(!fixture.shelf().entries[0].asked(), "still unconfirmed");
    }

    #[tokio::test]
    async fn the_persons_confirm_makes_an_entry_theirs_and_keeps_the_rest() {
        let fixture = Fixture::new();
        let before = Entry {
            permission_mode: Some("acceptEdits".into()),
            ..entry()
        };
        fixture.schedules.store().save(&before).expect("an entry");
        ran(&fixture, true, "confirm abcd")
            .await
            .expect("confirmed");
        let after = &fixture.shelf().entries[0];
        assert!(after.asked());
        assert_eq!(
            Entry {
                by: before.by,
                ..after.clone()
            },
            before,
            "only who asked changed"
        );
    }

    #[tokio::test]
    async fn a_word_it_does_not_take_is_refused() {
        let fixture = Fixture::new();
        let refused = ran(&fixture, true, "unknown abcd").await;
        assert_eq!(refused.expect_err("refused").code, ErrorCode::InvalidInput);
    }
}

#[cfg(test)]
#[path = "verbs_tests.rs"]
mod verbs_tests;
