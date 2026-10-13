//! Reading room debts through the shared fold.
pub use bingo_collab::mentions::*;
use bingo_sdk::{HostHandle, SessionId};
use crate::room;

/// nothing here: a debt is only ever what the journal says it is.
pub async fn of_room(host: &HostHandle, room: &SessionId) -> Vec<Mention> {
    room::read(host, room)
        .await
        .as_ref()
        .map(of_state)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
use bingo_sdk::{ItemId, ItemBody};
use crate::name::PARENT;
use jiff::Timestamp;
    use super::*;

    fn members() -> Vec<String> {
        ["reviewer", "scout"].map(str::to_string).to_vec()
    }

    /// A post at second `n` of the room's life.
    fn post(n: i64, author: &str, text: &str) -> Post {
        Post {
            id: ItemId::from_raw(format!("itm_{n}")),
            author: author.into(),
            at: Timestamp::from_second(n).expect("a timestamp"),
            text: text.into(),
        }
    }

    fn owed(posts: &[Post]) -> Vec<(String, String)> {
        mentions(posts, &members())
            .iter()
            .map(|m| (m.owed_by.said(), m.asker.clone()))
            .collect()
    }

    #[test]
    fn an_at_a_member_opens_a_debt_and_the_member_s_next_post_closes_it() {
        let asked = [post(1, "parent", "@reviewer can you look?")];
        assert_eq!(owed(&asked), [("reviewer".into(), "parent".into())]);

        let answered = [
            post(1, "parent", "@reviewer can you look?"),
            post(2, "reviewer", "anything at all"),
        ];
        assert!(owed(&answered).is_empty(), "speaking is the answer");
    }

    /// The parser table, as the old tree's reads.
    #[test]
    fn a_name_is_a_word_and_only_a_member_s() {
        let table = [
            ("@reviewer", true),
            ("@Reviewer,", true),
            ("hey @REVIEWER!", true),
            ("(@reviewer)", true),
            ("@reviewer-", true),
            ("line one\n@reviewer", true),
            ("mail@reviewer", false),
            ("x_@reviewer", false),
            ("@reviewers", false),
            ("@code-reviewer", false),
            ("@nobody", false),
            ("reviewer", false),
            ("@", false),
        ];
        for (text, hits) in table {
            let opened = !named(text, &members()).is_empty();
            assert_eq!(opened, hits, "{text:?}");
        }
    }

    #[test]
    fn a_hyphenated_member_is_one_word() {
        let members = ["code-reviewer".to_string()];
        assert_eq!(
            named("@code-reviewer, please", &members),
            [Owed::Member("code-reviewer".into())]
        );
    }

    #[test]
    fn a_debt_is_owed_in_the_roster_s_spelling_however_it_was_typed() {
        let mentions = mentions(&[post(1, "parent", "@REVIEWER ping")], &members());
        assert_eq!(mentions[0].owed_by, Owed::Member("reviewer".into()));
        assert_eq!(mentions[0].owed_by.chased(), Some("reviewer"));
    }

    #[test]
    fn a_post_that_answers_and_asks_settles_before_it_opens() {
        let posts = [
            post(1, "parent", "@scout what does the log say?"),
            post(2, "scout", "thanks @scout — @reviewer what about x?"),
        ];
        assert_eq!(
            owed(&posts),
            [("reviewer".into(), "scout".into())],
            "the scout's own debt closed and it may not open one on itself"
        );
    }

    #[test]
    fn one_post_owes_once_however_often_it_says_the_name() {
        let posts = [post(1, "parent", "@reviewer @reviewer @reviewer look")];
        assert_eq!(owed(&posts).len(), 1);
    }

    #[test]
    fn at_all_is_one_debt_against_the_room_that_any_other_member_closes() {
        let asked = [post(1, "reviewer", "@all stand-up in five")];
        assert_eq!(owed(&asked), [("@all".into(), "reviewer".into())]);
        assert_eq!(
            mentions(&asked, &members())[0].owed_by.chased(),
            None,
            "the sigil named nobody, so the chase names nobody"
        );

        let again = [
            post(1, "reviewer", "@all stand-up in five"),
            post(2, "reviewer", "anyone?"),
            post(3, "parent", "in a minute"),
        ];
        assert_eq!(
            owed(&again).len(),
            1,
            "neither the one who asked nor a session that is not a member closes it"
        );

        let closed = [
            post(1, "reviewer", "@all stand-up in five"),
            post(2, "scout", "on my way"),
        ];
        assert!(owed(&closed).is_empty());
    }

    /// A room that seats somebody called `all` spends the word on them: the
    /// sigil is what is left over when no member answers to it, so nobody's
    /// name is ever taken by it.
    #[test]
    fn a_member_named_all_takes_the_word_back_from_the_sigil() {
        let seated = ["all".to_string(), "scout".to_string()];
        assert_eq!(
            named("@all stand-up in five", &seated),
            [Owed::Member("all".into())],
            "the member, and no debt against the room"
        );
        assert_eq!(
            named("@all stand-up in five", &["scout".to_string()]),
            [Owed::Room],
            "and off that roster the word is the room's again"
        );
    }

    /// The roster is asked at the fold, never baked into the post: a member
    /// seated after `@all` was said answers it like any other.
    #[test]
    fn at_all_is_answered_by_a_member_the_room_gained_after_it_was_said() {
        let asked = [post(1, "reviewer", "@all stand-up in five")];
        let joined = ["reviewer", "scout", "latecomer"]
            .map(str::to_string)
            .to_vec();
        assert_eq!(mentions(&asked, &joined).len(), 1, "still standing");

        let answered = [
            post(1, "reviewer", "@all stand-up in five"),
            post(2, "latecomer", "here"),
        ];
        assert!(mentions(&answered, &joined).is_empty());
    }

    #[test]
    fn a_member_s_post_closes_every_debt_they_owe() {
        let posts = [
            post(1, "parent", "@reviewer one"),
            post(2, "scout", "@reviewer two"),
            post(3, "reviewer", "both, then"),
        ];
        assert!(owed(&posts).is_empty());
    }

    /// ADR-0028 §3: a rostered holder is a member to the fold, and the seat's
    /// own post — a root holder's signs `parent` — closes what it owes.
    #[test]
    fn the_holder_on_the_roster_owes_and_answers_like_any_member() {
        let seated = ["scout".to_string(), PARENT.to_string()];
        let asked = [post(1, "scout", "@parent what does the build say?")];
        assert_eq!(
            mentions(&asked, &seated)[0].owed_by,
            Owed::Member(PARENT.into())
        );

        let answered = [
            post(1, "scout", "@parent what does the build say?"),
            post(2, PARENT, "it is green"),
        ];
        assert!(
            mentions(&answered, &seated).is_empty(),
            "the seat's own post closes what it owes"
        );
        assert!(
            mentions(&asked, &["scout".to_string()]).is_empty(),
            "off the roster, `@parent` opens nothing (ADR-0028 §4)"
        );
    }

    #[test]
    fn a_room_nobody_is_in_owes_nothing() {
        assert!(mentions(&[post(1, "parent", "@reviewer look")], &[]).is_empty());
    }

    #[test]
    fn a_debt_carries_the_head_of_the_post_that_opened_it() {
        let long = format!("@reviewer {}", "x".repeat(200));
        let mentions = mentions(&[post(1, "parent", &long)], &members());
        assert_eq!(mentions[0].head.chars().count(), HEAD);
        assert!(mentions[0].head.ends_with('…'));
        assert_eq!(mentions[0].at, Timestamp::from_second(1).expect("a stamp"));
        assert_eq!(mentions[0].post, ItemId::from_raw("itm_1"));
    }

    #[test]
    fn a_head_is_the_first_line_of_it() {
        assert_eq!(head("  @reviewer look  \nand again"), "@reviewer look");
        assert_eq!(head(""), "");
    }

    #[test]
    fn only_a_user_item_in_the_journal_is_a_post() {
        use crate::tests::{item, posted_item};
        assert_eq!(
            Post::of(&posted_item("@reviewer look", Some("scout")))
                .expect("a post")
                .author,
            "scout"
        );
        assert_eq!(
            Post::of(&posted_item("look", None)).expect("a post").author,
            PARENT,
            "a post nobody signed came from the session the room hangs under"
        );
        assert_eq!(
            Post::of(&item(ItemBody::Assistant {
                text: "@reviewer look".into()
            })),
            None,
            "a room answers nobody, and what it does not say it does not owe"
        );
    }
}
