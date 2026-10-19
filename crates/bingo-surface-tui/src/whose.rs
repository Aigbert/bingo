//! Whose session a click on the transcript walks into (`walkTo`): the
//! teammate a call started or addressed, or the one a post's name signs for.
use crate::{face::Faces, layout::grid, tree::Tree};
use bingo_sdk::{Item, ItemBody, SessionId};
use unicode_width::UnicodeWidthStr;

/// Where on its block a click landed: the block's first line or not, and the
/// column within the transcript.
pub struct Spot {
    pub first_line: bool,
    pub column: usize,
}

pub fn walked(
    tree: &Tree,
    looks: &crate::tool_rows::Looks,
    item: &Item,
    spot: &Spot,
) -> Option<SessionId> {
    if let Some(session) = tree.spawned_by(&item.id) {
        return Some(session.clone());
    }
    let faces = Faces::of(tree);
    let name = match &item.body {
        ItemBody::ToolCall { name, input, .. } => input.get(looks.who(name)?)?.as_str()?,
        ItemBody::User { origin, .. } if on_name(spot, origin.principal.as_deref()?) => {
            origin.principal.as_deref()?
        }
        _ => return None,
    };
    faces.teammate(name).map(|row| row.id.clone())
}

/// A post's name sits on its first line, from the transcript's text column.
fn on_name(spot: &Spot, name: &str) -> bool {
    let start = grid::TX;
    let end = start + name.width().min(crate::transcript::post::GUT);
    spot.first_line && (start..end).contains(&spot.column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    fn looks() -> crate::tool_rows::Looks {
        crate::tool_rows::Looks::of(&[bingo_sdk::CatalogEntry {
            id: "SendMessage".into(),
            label: "SendMessage".into(),
            meta: serde_json::json!({"look": bingo_sdk::ToolLook::new('@', "send").who("to")}),
        }])
    }

    #[test]
    fn a_posts_name_and_a_call_to_a_teammate_walk_in() {
        let tree = folded_tree(vec![child_frame(1, announced("reviewer"))]);
        let on = |first_line, column| Spot { first_line, column };
        let said = post("itm_p", "reviewer", "the cut reads well");
        assert_eq!(
            walked(&tree, &looks(), &said, &on(true, grid::TX)),
            Some(child_id())
        );
        assert_eq!(walked(&tree, &looks(), &said, &on(true, 20)), None);
        assert_eq!(walked(&tree, &looks(), &said, &on(false, grid::TX)), None);
        let call = tool(
            "itm_s",
            "SendMessage",
            serde_json::json!({"to": "reviewer", "text": "look again"}),
            None,
            bingo_sdk::ItemStatus::Completed,
        );
        assert_eq!(
            walked(&tree, &looks(), &call, &on(false, 40)),
            Some(child_id())
        );
        let stranger = post("itm_q", "nobody", "hello");
        assert_eq!(
            walked(&tree, &looks(), &stranger, &on(true, grid::TX)),
            None
        );
    }
}
