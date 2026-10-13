//! SendMessage against the rooms writer, across the real service seam.
#[allow(dead_code)]
#[path = "../../bingo-agents/src/tests.rs"]
mod support;

fn tool_context(
    session: &bingo_sdk::SessionId,
    call: std::sync::Arc<support::Recorder>,
) -> bingo_sdk::ToolContext {
    let cx = support::tool_context(session, call);
    let writer = std::sync::Arc::new(bingo_rooms::RoomWriter::new(cx.host.clone()));
    cx.host
        .open_service(
            bingo_services::room_post::KEY.name(),
            bingo_services::room_post::wire(writer),
        )
        .unwrap();
    cx
}

#[cfg(test)]
mod tests {
    use bingo_agents::MessageTool;
    use bingo_collab::{name as names, room as rooms};
    use bingo_sdk::{Delivery, Input, SessionId, Subject, Tool, ToolError, ToolOutput, View};
    use serde_json::Value;
    use std::path::Path;
    const SEND_MESSAGE: &str = "SendMessage";
    const NOT_A_ROOM: &str = "`again` repeats a bounced room post; write to an agent with `text`";
    use super::tool_context;
    use crate::support::{Fleet, Recorder};
    use bingo_sdk::ContentPart;
    use serde_json::json;
    use std::sync::Arc;

    /// A root with one child and one room under it, as a person's session
    /// holds both.
    fn fleet() -> (Fleet, bingo_sdk::SessionId) {
        let fleet = Fleet::default();
        let root = fleet.root();
        fleet.child(&root, "reviewer");
        fleet.room(&root, "#design");
        (fleet, root)
    }

    async fn send(to: &str) -> (ToolOutput, Arc<Recorder>) {
        let (fleet, root) = fleet();
        let host = Recorder::new(&fleet);
        let cx = tool_context(&root, host.clone());
        let out = MessageTool
            .call(json!({ "to": to, "text": "look again" }), &cx)
            .await
            .expect("a message this crate can deliver");
        (out, host)
    }

    #[tokio::test]
    async fn a_message_wakes_the_child_and_says_who_wrote_it() {
        let (out, host) = send("reviewer").await;
        assert!(!out.is_error);
        let delivered = host.delivered();
        assert_eq!(delivered.len(), 1);
        let (_, input, delivery) = &delivered[0];
        assert_eq!(*delivery, Delivery::Wake, "an idle target starts a turn");
        let Input::Text { text, origin, .. } = input else {
            panic!("a peer delivers text");
        };
        assert_eq!(text, "look again");
        assert_eq!(origin.principal.as_deref(), Some(names::PARENT));
    }

    /// The address space of ADR-0024 §1, through the tool the model calls.
    #[tokio::test]
    async fn a_teammate_is_written_to_by_name() {
        let fleet = Fleet::default();
        let root = fleet.root();
        let builder = fleet.child(&root, "builder");
        let reviewer = fleet.child(&root, "reviewer");
        let host = Recorder::new(&fleet);

        let out = MessageTool
            .call(
                json!({ "to": "reviewer", "text": "look again" }),
                &tool_context(&builder, host.clone()),
            )
            .await
            .expect("a message this crate can deliver");
        assert!(!out.is_error);
        let delivered = host.delivered();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].0, reviewer);
        let Input::Text { origin, .. } = &delivered[0].1 else {
            panic!("a peer delivers text");
        };
        assert_eq!(origin.principal.as_deref(), Some("builder"));
    }

    /// The block lane (ADR-0013 §2): the same delivery a person reads, asserted
    /// as the value it is.
    #[tokio::test]
    async fn the_card_says_where_it_went_and_when_it_will_be_read() {
        let (sent, _) = send("reviewer").await;
        assert_eq!(
            sent.display,
            Some(View::KeyValue {
                rows: vec![
                    ("to".into(), "reviewer".into()),
                    ("from".into(), "parent".into()),
                    (
                        "read".into(),
                        "as its next turn, or mid-run if it is working".into()
                    ),
                ]
            })
        );

        let (posted, _) = send("#design").await;
        assert_eq!(
            posted.display,
            Some(View::KeyValue {
                rows: vec![
                    ("to".into(), "#design".into()),
                    ("from".into(), "parent".into()),
                    (
                        "read".into(),
                        "at each seat's next turn; an @ wakes now".into()
                    ),
                ]
            }),
            "a room is read by everyone in it, not taken up as a turn"
        );
    }

    #[tokio::test]
    async fn a_name_nobody_has_is_an_error_result_the_model_can_correct() {
        let (out, host) = send("nobody").await;
        assert!(out.is_error);
        assert!(host.delivered().is_empty());
    }

    /// A room has no turns to promise anything about, so the receipt says
    /// what did happen and nothing more.
    #[tokio::test]
    async fn a_message_to_a_room_is_a_post() {
        let (out, host) = send("#design").await;
        assert!(!out.is_error);
        assert_eq!(out.parts[0].as_text(), Some("Posted to #design."));
        assert_eq!(host.delivered().len(), 1);
    }

    #[test]
    fn it_reads_only_and_names_the_agent_a_rule_may_match() {
        assert_eq!(MessageTool.spec().name, "SendMessage");
        let traits = MessageTool.traits(&Value::Null);
        assert!(traits.read_only && traits.trusted && !traits.concurrency_safe);
        assert_eq!(
            MessageTool.subjects(&json!({ "to": "reviewer", "text": "x" }), Path::new("/")),
            [Subject::Name {
                name: "reviewer".into()
            }]
        );
    }

    /// The rule of ADR-0024 §4, and the one word a bounce costs (ADR-0053
    /// §6), where the model reads them.
    #[test]
    fn the_description_says_a_direct_message_owes_nothing() {
        let description = MessageTool.spec().description;
        let description = description.as_str();
        assert!(description.contains("Beside you"), "{description}");
        assert!(description.contains("@name"), "{description}");
        assert!(
            description.contains("a direct message is not"),
            "{description}"
        );
        assert!(description.contains("`again: true`"), "{description}");
    }

    /// A room with one member beside the caller, holding a post of that
    /// member's already. The root is off the roster, so it keeps no cursor
    /// there and posts blind (ADR-0025, consequences): the first thing it
    /// writes bounces, which is where every scenario below starts.
    fn spoken_for() -> (Fleet, SessionId, SessionId) {
        let fleet = Fleet::default();
        let root = fleet.root();
        let room = fleet.room(&root, "#design");
        fleet.child(&root, "scout");
        fleet.post(&room, "the build is green", Some("scout"));
        (fleet, root, room)
    }

    /// One call, journaled where a completed one lands: the caller's own
    /// history, which is where the next `again` looks for its draft.
    async fn called(
        fleet: &Fleet,
        caller: &SessionId,
        host: Arc<Recorder>,
        input: Value,
    ) -> Result<ToolOutput, ToolError> {
        let out = MessageTool
            .call(input.clone(), &tool_context(caller, host))
            .await;
        if let Ok(out) = &out {
            fleet.called(caller, SEND_MESSAGE, input, out.clone());
        }
        out
    }

    fn said(out: &ToolOutput) -> String {
        out.parts.iter().filter_map(ContentPart::as_text).collect()
    }

    /// What one delivery carried, and whose name it arrived under.
    fn delivered(host: &Recorder) -> Vec<(String, Option<String>)> {
        host.delivered()
            .into_iter()
            .filter_map(|(_, input, _)| match input {
                Input::Text { text, origin, .. } => Some((text, origin.principal)),
                _ => None,
            })
            .collect()
    }

    /// The whole of ADR-0053 §6: the draft a bounce handed back is the
    /// caller's own bounced call, so one word posts it again, in the words it
    /// was written in and over the caller's own name.
    #[tokio::test]
    async fn a_bounced_post_is_posted_again_by_one_word() {
        let (fleet, root, _) = spoken_for();
        let host = Recorder::new(&fleet);
        let post = json!({ "to": "#design", "text": "stand-up in five" });

        let bounced = called(&fleet, &root, host.clone(), post)
            .await
            .expect("a post this crate can judge");
        assert!(bounced.is_error, "the root never heard the scout's post");
        assert!(delivered(&host).is_empty(), "a bounced post never landed");

        let posted = called(
            &fleet,
            &root,
            host.clone(),
            json!({ "to": "#design", "again": true }),
        )
        .await
        .expect("a post this crate can judge");
        assert!(!posted.is_error, "{}", said(&posted));
        assert_eq!(said(&posted), "Posted to #design.");
        assert_eq!(
            delivered(&host),
            [(
                "stand-up in five".to_string(),
                Some(names::PARENT.to_string())
            )]
        );
    }

    /// An `again` is judged like any other post (ADR-0053 §6): one that landed
    /// while the caller was repeating itself bounces the repeat, carrying only
    /// what is newly missed — and the next `again` posts the same draft.
    #[tokio::test]
    async fn a_post_that_landed_since_bounces_the_repeat_and_the_next_one_lands() {
        let (fleet, root, room) = spoken_for();
        let host = Recorder::new(&fleet);
        let post = json!({ "to": "#design", "text": "stand-up in five" });
        let again = json!({ "to": "#design", "again": true });

        called(&fleet, &root, host.clone(), post)
            .await
            .expect("a post this crate can judge");
        fleet.post(&room, "and the tests pass", Some("scout"));

        let bounced = called(&fleet, &root, host.clone(), again.clone())
            .await
            .expect("a post this crate can judge");
        assert!(bounced.is_error, "the scout spoke again first");
        let text = said(&bounced);
        assert!(text.contains("scout: and the tests pass"), "{text}");
        assert!(
            !text.contains("the build is green"),
            "the first bounce read that one out already: {text}"
        );
        assert!(delivered(&host).is_empty());

        let posted = called(&fleet, &root, host.clone(), again)
            .await
            .expect("a post this crate can judge");
        assert!(!posted.is_error, "{}", said(&posted));
        assert_eq!(
            delivered(&host),
            [(
                "stand-up in five".to_string(),
                Some(names::PARENT.to_string())
            )],
            "an `again` carries no text of its own, so the draft is still the first one"
        );
    }

    /// `text` and `again` are two different things to write, and a call that
    /// says neither says nothing: both are input the caller corrects.
    #[tokio::test]
    async fn a_call_that_says_both_or_neither_is_refused_as_input() {
        let (fleet, root, _) = spoken_for();
        let host = Recorder::new(&fleet);
        let cx = tool_context(&root, host.clone());

        let both = MessageTool
            .call(
                json!({ "to": "#design", "text": "stand-up", "again": true }),
                &cx,
            )
            .await;
        let Err(ToolError::InvalidInput(said)) = both else {
            panic!("two things to write is not one post: {both:?}");
        };
        assert_eq!(said, "give `text` or `again`, not both");

        let neither = MessageTool.call(json!({ "to": "#design" }), &cx).await;
        assert!(
            matches!(&neither, Err(ToolError::InvalidInput(_))),
            "{neither:?}"
        );
        assert!(delivered(&host).is_empty());
    }

    /// Only a room bounces, so only a room holds a draft to repeat.
    #[tokio::test]
    async fn again_to_an_agent_is_refused_as_input() {
        let (fleet, root, _) = spoken_for();
        let host = Recorder::new(&fleet);

        let out = MessageTool
            .call(
                json!({ "to": "scout", "again": true }),
                &tool_context(&root, host.clone()),
            )
            .await;
        let Err(ToolError::InvalidInput(said)) = out else {
            panic!("an agent hands nothing back to repeat: {out:?}");
        };
        assert_eq!(said, NOT_A_ROOM);
        assert!(delivered(&host).is_empty());
    }

    /// An `again` with no bounce behind it is the model's own mistake to
    /// correct in the turn it is in, so it is an error result and not a
    /// refusal of the input: what is missing is a fact of the journal.
    #[tokio::test]
    async fn again_with_nothing_to_repeat_delivers_nothing() {
        let (fleet, root, _) = spoken_for();
        let host = Recorder::new(&fleet);

        let out = called(
            &fleet,
            &root,
            host.clone(),
            json!({ "to": "#design", "again": true }),
        )
        .await
        .expect("a post this crate can judge");
        assert!(out.is_error);
        assert_eq!(
            said(&out),
            "nothing to post again in #design: no post of yours bounced there"
        );
        assert!(delivered(&host).is_empty());
    }

    /// ADR-0053 §4: a closed room takes no post, in either spelling, and the
    /// caller is told what to do instead.
    #[tokio::test]
    async fn nothing_lands_in_a_closed_room() {
        let (fleet, root, room) = spoken_for();
        fleet.extended(
            &room,
            rooms::PLUGIN,
            rooms::CLOSED,
            json!({ "at": "2026-09-10T09:00:00Z", "by": "parent", "why": "it is settled" }),
        );
        let host = Recorder::new(&fleet);

        let out = called(
            &fleet,
            &root,
            host.clone(),
            json!({ "to": "#design", "text": "stand-up in five" }),
        )
        .await
        .expect("a post this crate can judge");
        assert!(out.is_error);
        assert_eq!(
            said(&out),
            "#design is closed; nothing lands there — open another room"
        );

        let repeated = called(
            &fleet,
            &root,
            host.clone(),
            json!({ "to": "#design", "again": true }),
        )
        .await
        .expect("a post this crate can judge");
        assert!(repeated.is_error, "{}", said(&repeated));
        assert!(delivered(&host).is_empty());
    }
    /// End to end through the tool, on a fleet whose room holds a post the
    /// caller never heard: the author is never behind on its own post, and
    /// the one that was not there is.
    #[tokio::test]
    async fn a_stale_post_bounces_and_lands_nothing() {
        let fleet = Fleet::default();
        let root = fleet.root();
        let room = fleet.room(&root, "#design");
        let scout = fleet.child(&root, "scout");
        fleet.post(&room, "the build is green", Some("scout"));
        let host = Recorder::new(&fleet);

        let out = posted(&scout, host.clone()).await;
        assert!(!out.is_error, "the author is not behind on its own post");
        assert_eq!(host.delivered().len(), 1);

        let out = posted(&root, host.clone()).await;
        assert!(out.is_error, "the root never heard the scout's post");
        let text = out.parts[0].as_text().unwrap_or_default();
        assert!(text.contains("scout: the build is green"), "{text}");
        assert_eq!(host.delivered().len(), 1, "a bounced post never landed");
    }

    async fn posted(caller: &bingo_sdk::SessionId, host: std::sync::Arc<Recorder>) -> ToolOutput {
        bingo_agents::MessageTool
            .call(
                json!({ "to": "#design", "text": "stand-up in five" }),
                &tool_context(caller, host),
            )
            .await
            .expect("a post this crate can judge")
    }
}
