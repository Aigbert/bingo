//! Person-authored text shared by history projections.
use crate::{ContentPart, Item, ItemBody, Origin, Voice};

/// The surfaces a person types through. Before producers stamped a voice,
/// these were the only ones whose lines were the person's own.
pub const PERSON_SURFACES: [&str; 5] = ["tui", "print", "rpc", "acp", "channels"];

/// An explicit voice wins. An unstamped origin fails closed: it is the
/// person's only from a person surface, and anything else — generated
/// context, a kernel prompt, a name this list has never heard — is quiet.
pub fn effective_voice(origin: &Origin) -> Voice {
    match origin.voice {
        Some(voice) => voice,
        None if PERSON_SURFACES.contains(&origin.surface.as_str()) => Voice::Said,
        None => Voice::Quiet,
    }
}

/// The first text part spoken by the person, borrowed without joining parts.
/// `first_ask` is the first line of this text, not the whole ask.
pub fn asked(item: &Item) -> Option<&str> {
    match &item.body {
        ItemBody::User { parts, origin } if effective_voice(origin) == Voice::Said => {
            parts.iter().find_map(ContentPart::as_text)
        }
        _ => None,
    }
}

/// The longest first ask a summary carries.
const FIRST_ASK_CHARS: usize = 200;

/// What a summary's `first_ask` holds for this item: the first line of what
/// was [`asked`], trimmed and capped. The live fold, the store's counter and
/// the store's rebuild all derive it here.
pub fn first_ask(item: &Item) -> Option<String> {
    let line = first_line(asked(item)?).unwrap_or_default();
    Some(line.trim().chars().take(FIRST_ASK_CHARS).collect())
}

pub fn first_line(text: &str) -> Option<&str> {
    text.lines().next()
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
        assert_eq!(first_ask(&item).as_deref(), Some("first"));
        item.body = ItemBody::User {
            parts: vec![ContentPart::text("字".repeat(300))],
            origin: Origin::surface("tui"),
        };
        assert_eq!(first_ask(&item).map(|ask| ask.chars().count()), Some(200));
        if let ItemBody::User { origin, .. } = &mut item.body {
            origin.voice = Some(Voice::Note);
        }
        assert_eq!(asked(&item), None);
        assert_eq!(first_ask(&item), None);
        for surface in ["command", "wake", "plugin:x", "contributor:git"] {
            item.body = ItemBody::User {
                parts: vec![ContentPart::text("not the person's")],
                origin: Origin::surface(surface),
            };
            assert_eq!(first_ask(&item), None, "{surface}");
        }
        item.body = ItemBody::Assistant {
            text: "answer".into(),
        };
        assert_eq!(asked(&item), None);
    }
}
