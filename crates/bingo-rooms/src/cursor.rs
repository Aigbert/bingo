//! Cursor writes; readings are shared.
pub use bingo_collab::cursor::*;
use bingo_sdk::{HostHandle, ItemId, KernelError, SessionId};
use crate::PLUGIN;

/// Move a seat's cursor to a post it has now read.
pub async fn advance(
    host: &HostHandle,
    room: &SessionId,
    member: &str,
    post: &ItemId,
) -> Result<(), KernelError> {
    host.extend(room, PLUGIN, &kind(member), payload(post))
        .await
}

#[cfg(test)]
mod tests {
use bingo_sdk::SessionState;
use crate::mentions::Post;
use serde_json::{Value, json};
    use super::*;
    use crate::room;
    use crate::tests::{Fleet, posted_item, summary};
    use jiff::Timestamp;

    fn post(n: i64, author: &str) -> Post {
        Post {
            id: ItemId::from_raw(format!("itm_{n}")),
            author: author.into(),
            at: Timestamp::from_second(n).expect("a timestamp"),
            text: format!("post {n}"),
        }
    }

    fn ids(posts: &[Post]) -> Vec<&str> {
        posts.iter().map(|post| post.id.as_str()).collect()
    }

    /// The persisted shape, written down: a cursor is a payload another process
    /// — and another crate's serial rule — reads back by hand (ADR-0034 §2).
    #[test]
    fn a_cursor_is_one_post_id_under_a_kind_of_its_own() {
        assert_eq!(kind("Scout"), "cursor:scout");
        assert_eq!(
            payload(&ItemId::from_raw("itm_7")),
            json!({ "post": "itm_7" })
        );

        const WRITTEN: &str = r#"{"post":"itm_7"}"#;
        let payload: Value = serde_json::from_str(WRITTEN).expect("a cursor payload");
        assert_eq!(stored(&payload), Some(ItemId::from_raw("itm_7")));
        assert_eq!(stored(&json!({})), None, "a payload that is not one");
        assert_eq!(stored(&Value::Null), None);
    }

    /// The whole of the split, as one table: what landed after the cursor.
    #[test]
    fn what_is_unread_is_what_landed_after_the_cursor() {
        let posts = [post(1, "scout"), post(2, "reviewer"), post(3, "scout")];

        let read_two = ItemId::from_raw("itm_2");
        assert_eq!(ids(unread(&posts, Some(&read_two))), ["itm_3"]);
        assert!(
            unread(&posts, Some(&ItemId::from_raw("itm_3"))).is_empty(),
            "a seat at the head has nothing to read"
        );
        assert_eq!(
            ids(unread(&posts, None)),
            ["itm_1", "itm_2", "itm_3"],
            "a seat with no cursor has read none of it"
        );
        assert_eq!(
            ids(unread(&posts, Some(&ItemId::from_raw("itm_gone")))),
            ["itm_1", "itm_2", "itm_3"],
            "and a cursor the room does not hold measures nothing"
        );
    }

    /// A kind per seat, so a room with two of them keeps two cursors and
    /// neither writes over the other.
    #[tokio::test]
    async fn a_room_s_own_journal_says_where_each_seat_has_read_up_to() {
        let fleet = Fleet::default();
        let root = fleet.root();
        let id = fleet.room(&root, "design");
        let host = fleet.handle();

        advance(&host, &id, "scout", &ItemId::from_raw("itm_2"))
            .await
            .expect("a cursor this crate can write");
        advance(&host, &id, "Reviewer", &ItemId::from_raw("itm_9"))
            .await
            .expect("a cursor this crate can write");

        let state = room::read(&host, &id).await.expect("the room");
        assert_eq!(of_state(&state, "scout"), Some(ItemId::from_raw("itm_2")));
        assert_eq!(
            of_state(&state, "reviewer"),
            Some(ItemId::from_raw("itm_9")),
            "one seat's cursor is not another's, in any case"
        );
        assert_eq!(of_state(&state, "nobody"), None);
    }

    /// What a seat is handed to read: everyone else's posts after its cursor,
    /// and the head that its own post still moves.
    #[test]
    fn a_seat_reads_the_others_and_its_own_post_still_moves_the_cursor() {
        let mut room = SessionState::new(summary("ses_room", Some("#design"), None));
        room.transcript = vec![
            posted_item("first", Some("reviewer")),
            posted_item("mine", Some("scout")),
        ]
        .into_iter()
        .collect();

        let unread = Unread::of(&room, "scout");
        assert_eq!(
            unread
                .posts
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>(),
            ["first"],
            "a seat is never handed its own post"
        );
        assert_eq!(
            unread.head.as_ref(),
            Some(&room.transcript.items()[1].id),
            "and the cursor still moves past it"
        );
    }
}
