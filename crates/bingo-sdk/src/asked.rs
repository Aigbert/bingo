//! Person-authored text shared by history projections.
use crate::{ContentPart, Item, ItemBody, Voice};

/// Return the person's full text when the item's voice is Said (person input),
/// otherwise None. `first_ask` is the first line of this text, not the whole ask.
/// User input stores its text in the first text part; other parts are attachments.
pub fn asked(item: &Item) -> Option<&str> {
    let ItemBody::User { parts, origin } = &item.body else {
        return None;
    };
    if origin.is_context() || origin.voice.unwrap_or(Voice::Said) != Voice::Said {
        return None;
    }
    parts.iter().find_map(ContentPart::as_text)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    #[test]
    fn a_persons_full_text_is_an_ask() {
        let mut item: Item = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../fixtures/record/event/itemCompleted.json"
            ))
            .unwrap()["item"]
                .clone(),
        )
        .unwrap();
        item.body = ItemBody::User {
            parts: vec![ContentPart::text("  first\nsecond")],
            origin: Origin::surface("tui"),
        };
        assert_eq!(asked(&item), Some("  first\nsecond"));
        if let ItemBody::User { origin, .. } = &mut item.body {
            origin.voice = Some(Voice::Note);
        }
        assert_eq!(asked(&item), None);
        item.body = ItemBody::Assistant {
            text: "answer".into(),
        };
        assert_eq!(asked(&item), None);
    }
}
