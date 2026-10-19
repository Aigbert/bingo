//! One conversion from a delivered input to its journaled user item.
use bingo_sdk::{ContentPart, Input, Item, ItemBody};

pub(crate) fn item(input: Input, mint: impl FnOnce(ItemBody) -> Item) -> Option<Item> {
    let Input::Text {
        text,
        images,
        origin,
        headline,
        ..
    } = input
    else {
        return None;
    };
    let mut parts = Vec::with_capacity(images.len() + 1);
    // An image-only ask has no empty text part (ADR-0040 §3).
    if !text.is_empty() {
        parts.push(ContentPart::text(text));
    }
    parts.extend(images.into_iter().map(ContentPart::Image));
    let mut item = mint(ItemBody::User { parts, origin });
    if let Some(headline) = headline {
        item.set_headline(headline);
    }
    Some(item)
}
