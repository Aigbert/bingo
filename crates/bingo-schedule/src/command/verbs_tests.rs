use super::*;
use crate::{entry::tests::entry, tests::Fixture};
#[tokio::test]
async fn verbs_refuse_unasked_and_disabled_and_resolve_prefixes() {
    let f = Fixture::new();
    let command = ScheduleCommand::new(f.schedules.clone());
    for verb in [
        "add every 2h -- check",
        "confirm abcd",
        "pause abcd",
        "resume abcd",
        "run abcd",
        "forget abcd",
    ] {
        assert!(
            command
                .run(verb, &f.command())
                .await
                .unwrap_err()
                .message
                .contains("person")
        );
    }
    f.schedules.store().save(&entry()).unwrap();
    bingo_sdk::render::under_input(true, async {
        command.run("pause abcd", &f.command()).await.unwrap();
        assert!(!f.shelf().entries[0].enabled);
        assert!(
            command
                .run("run abcd", &f.command())
                .await
                .unwrap_err()
                .message
                .contains("resume first")
        );
        command.run("resume abcd", &f.command()).await.unwrap();
        assert!(f.shelf().entries[0].enabled);
        // Failed delivery leaves the occurrence unspent.
        assert!(command.run("run abcd", &f.command()).await.is_err());
        assert!(f.shelf().entries[0].last_fired.is_none());
        f.schedules
            .store()
            .save(&crate::Entry {
                id: "abcd9999".into(),
                ..entry()
            })
            .unwrap();
        assert!(
            command
                .run("forget abcd", &f.command())
                .await
                .unwrap_err()
                .message
                .contains("could be any")
        );
        command.run("forget abcd1234", &f.command()).await.unwrap();
        assert_eq!(f.shelf().entries.len(), 1);
        assert!(
            command
                .run("add every 2h check", &f.command())
                .await
                .is_err()
        );
    })
    .await;
}
