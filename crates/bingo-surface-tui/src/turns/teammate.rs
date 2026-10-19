//! Whose step a call is: the teammate it started, or the one it addressed by
//! name, and when that teammate first posted back after it.
use crate::transcript::Rows;
use crate::tree::{self, Agents};
use bingo_sdk::component::Teammate;
use bingo_sdk::{Item, ItemBody, SessionId};

pub fn of(item: &Item, agents: &Agents<'_>, rows: &Rows<'_>) -> Option<Teammate> {
    let (id, name) = addressed(item, agents, rows)?;
    Some(Teammate {
        ink: rows.faces.ink(&id).into(),
        reported_at: reported(item, &name, rows),
        name,
    })
}

/// A spawned session is the call's own; otherwise the look names the field.
fn addressed(item: &Item, agents: &Agents<'_>, rows: &Rows<'_>) -> Option<(SessionId, String)> {
    if let Some(child) = agents.get(&item.id) {
        return Some((child.summary.id.clone(), tree::name(child)));
    }
    let ItemBody::ToolCall { name, input, .. } = &item.body else {
        return None;
    };
    let to = input.get(rows.tools.who(name)?)?.as_str()?;
    let row = rows.faces.teammate(to)?;
    Some((row.id.clone(), to.to_owned()))
}

/// The first post signed by `name` after the call.
fn reported(item: &Item, name: &str, rows: &Rows<'_>) -> Option<i64> {
    rows.state
        .transcript
        .items()
        .iter()
        .skip_while(|later| later.id != item.id)
        .find(|later| signed(later, name))
        .map(|post| post.started_at.as_millisecond())
}

fn signed(item: &Item, name: &str) -> bool {
    matches!(&item.body, ItemBody::User { origin, .. }
        if origin.principal.as_deref() == Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::{CatalogEntry, Event, ItemStatus, Reads, ToolLook};
    use serde_json::json;

    fn item_frame(seq: u64, item: bingo_sdk::Item) -> bingo_sdk::Frame {
        frame(seq, Event::ItemCompleted { item })
    }

    fn send_message() -> crate::tool_rows::Looks {
        let look = ToolLook::new('@', "send")
            .target("text", Reads::Text)
            .who("to");
        crate::tool_rows::Looks::of(&[CatalogEntry {
            id: "SendMessage".into(),
            label: "SendMessage".into(),
            meta: json!({ "look": look }),
        }])
    }

    /// A home that spawned `reviewer` at `itm_1`, then sent it `itm_2`.
    fn home(reported: bool) -> crate::tree::Tree {
        let mut frames = vec![
            item_frame(
                1,
                tool(
                    "itm_1",
                    "SpawnAgent",
                    json!({"prompt": "review the diff"}),
                    None,
                    ItemStatus::Completed,
                ),
            ),
            child_frame(1, announced("reviewer")),
            item_frame(
                2,
                tool(
                    "itm_2",
                    "SendMessage",
                    json!({"to": "reviewer", "text": "look again"}),
                    None,
                    ItemStatus::Completed,
                ),
            ),
        ];
        if reported {
            frames.push(frame(
                3,
                Event::ItemCompleted {
                    item: post("itm_3", "reviewer", "finished.\n\nlooks fine"),
                },
            ));
        }
        folded_tree(frames)
    }

    fn teammate(tree: &crate::tree::Tree, id: &str) -> Option<Teammate> {
        let (folds, pictures, linked) = Default::default();
        let mut rows =
            Rows::of(tree.viewed(), 80, &folds, &pictures, &linked, scene().1).in_tree(tree);
        rows.tools = send_message();
        let item = tree
            .viewed()
            .transcript
            .get(&bingo_sdk::ItemId::from_raw(id))?;
        of(item, &tree.agents(), &rows)
    }

    #[test]
    fn a_spawn_and_a_message_name_the_teammate_in_its_ink() {
        let tree = home(false);
        let ink = crate::face::Faces::of(&tree).who("reviewer").to_owned();
        assert!(ink.starts_with("crew"), "{ink}");
        for id in ["itm_1", "itm_2"] {
            let named = teammate(&tree, id).expect("a teammate");
            assert_eq!(
                (named.name.as_str(), named.ink.as_str()),
                ("reviewer", ink.as_str())
            );
            assert_eq!(named.reported_at, None, "{id} has not been answered");
        }
    }

    #[test]
    fn a_post_signed_by_the_teammate_reports_the_steps_before_it() {
        let tree = home(true);
        let posted = tree
            .viewed()
            .transcript
            .get(&bingo_sdk::ItemId::from_raw("itm_3"))
            .map(|post| post.started_at.as_millisecond());
        for id in ["itm_1", "itm_2"] {
            assert_eq!(teammate(&tree, id).and_then(|t| t.reported_at), posted);
        }
    }

    #[test]
    fn a_message_to_nobody_on_the_crew_is_an_ordinary_step() {
        let mut tree = home(false);
        tree.fold.apply(&item_frame(
            4,
            tool(
                "itm_4",
                "SendMessage",
                json!({"to": "#design", "text": "hello"}),
                None,
                ItemStatus::Completed,
            ),
        ));
        assert!(teammate(&tree, "itm_4").is_none());
    }
}
