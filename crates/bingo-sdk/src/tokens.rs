//! The cheap token estimate every measuring site shares (ADR-0006): ~4 ASCII
//! chars or one non-ASCII char per token, pixels / 750 per measured image, tool
//! schemas as their JSON. The kernel anchors it on the server's count; a
//! plugin that cuts uses the same rule so its `before` and `after` mean what
//! the kernel's numbers mean.

use crate::event::{Item, ItemBody};
use crate::model::{ContentPart, Message, SystemBlock, ToolSpec};

pub const IMAGE_TOKENS: u64 = 1_600;

pub fn text(s: &str) -> u64 {
    let (ascii, other) = s.chars().fold((0u64, 0u64), |(a, o), c| {
        if c.is_ascii() { (a + 1, o) } else { (a, o + 1) }
    });
    ascii.div_ceil(4) + other
}

pub fn parts(parts: &[ContentPart]) -> u64 {
    parts
        .iter()
        .map(|p| match p {
            ContentPart::Text { text: t } | ContentPart::Reasoning { text: t, .. } => text(t),
            ContentPart::Image(image) => match (image.width, image.height) {
                (Some(width), Some(height)) => (u64::from(width) * u64::from(height)).div_ceil(750),
                _ => IMAGE_TOKENS,
            },
            ContentPart::ToolUse { name, input, .. } => text(name) + text(&input.to_string()),
            ContentPart::ToolResult { parts: inner, .. } => self::parts(inner),
        })
        .fold(0, u64::saturating_add)
}

pub fn blocks(blocks: &[SystemBlock]) -> u64 {
    blocks
        .iter()
        .map(|b| text(&b.text))
        .fold(0, u64::saturating_add)
}

/// One item's own content, before the fold adds its wrappers.
pub fn item(item: &Item) -> u64 {
    match &item.body {
        ItemBody::User { parts: p, .. } => parts(p),
        ItemBody::Assistant { text: t } | ItemBody::Reasoning { text: t, .. } => text(t),
        ItemBody::ToolCall {
            name,
            input,
            output,
            ..
        } => {
            let result = output.as_ref().map(|o| parts(&o.parts)).unwrap_or(0);
            text(name)
                .saturating_add(text(&input.to_string()))
                .saturating_add(result)
        }
        ItemBody::Compaction { summary, .. } => text(summary),
        ItemBody::Interruption { marker } => text(marker),
        ItemBody::QuestionAnswer {
            question, answer, ..
        } => text(question) + text(answer),
        _ => 0,
    }
}

pub fn items(items: &[Item]) -> u64 {
    items.iter().map(item).fold(0, u64::saturating_add)
}

/// A whole request as the provider would count it.
pub fn estimate(system: &[SystemBlock], messages: &[Message], tools: &[ToolSpec]) -> u64 {
    let mut n = blocks(system);
    for m in messages {
        n = n.saturating_add(parts(&m.parts));
    }
    for t in tools {
        n = n.saturating_add(
            text(&t.name) + text(&t.description) + text(&t.input_schema.to_string()),
        );
    }
    n
}

/// Four disjoint estimates of a request, including pictures nested in results.
pub fn composition(
    system: &[SystemBlock],
    messages: &[Message],
    tools: &[ToolSpec],
) -> [crate::event::ContextPart; 4] {
    let pictures = messages
        .iter()
        .map(|m| picture_tokens(&m.parts))
        .fold(0, u64::saturating_add);
    let chat = messages
        .iter()
        .map(|m| parts(&m.parts))
        .fold(0, u64::saturating_add)
        .saturating_sub(pictures);
    use crate::ContextKind::*;
    [
        (System, blocks(system)),
        (Tools, estimate(&[], &[], tools)),
        (Chat, chat),
        (Pictures, pictures),
    ]
    .map(|(name, tokens)| crate::event::ContextPart { name, tokens })
}

fn picture_tokens(content: &[ContentPart]) -> u64 {
    content
        .iter()
        .map(|part| match part {
            ContentPart::Image(_) => parts(std::slice::from_ref(part)),
            ContentPart::ToolResult { parts, .. } => picture_tokens(parts),
            _ => 0,
        })
        .fold(0, u64::saturating_add)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Image;

    #[test]
    fn four_ascii_characters_are_one_token_and_a_cjk_character_is_one() {
        assert_eq!(text("abcd"), 1);
        assert_eq!(text("abcde"), 2);
        assert_eq!(text("汉字"), 2);
        assert_eq!(text(""), 0);
    }

    #[test]
    fn an_image_costs_a_flat_sixteen_hundred_wherever_it_sits() {
        let image = ContentPart::Image(Image {
            media_type: "image/png".into(),
            data: "aaaa".into(),
            width: None,
            height: None,
            path: None,
        });
        assert_eq!(parts(std::slice::from_ref(&image)), IMAGE_TOKENS);
        assert_eq!(
            parts(&[ContentPart::ToolResult {
                tool_use_id: "c".into(),
                parts: vec![image],
                is_error: false,
            }]),
            IMAGE_TOKENS
        );
    }

    #[test]
    fn a_request_counts_its_system_messages_and_tool_schemas() {
        let system = [SystemBlock {
            text: "abcdefgh".into(),
            cache: false,
        }];
        let messages = [Message::text(crate::model::Role::User, "abcd")];
        let tools = [ToolSpec {
            look: None,
            name: "Read".into(),
            description: "read".into(),
            input_schema: serde_json::json!({}),
            meta: Default::default(),
        }];
        assert_eq!(estimate(&system, &messages, &tools), 2 + 1 + 1 + 1 + 1);
    }
}

#[cfg(test)]
mod pixel_contract {
    #[test]
    fn dimensions_set_the_picture_estimate() {
        let image = serde_json::from_value(serde_json::json!({
            "type": "image", "mediaType": "image/png", "data": "eA==",
            "width": 2000, "height": 2000
        }))
        .unwrap();
        assert_eq!(super::parts(&[image]), 4_000_000u64.div_ceil(750));
    }
}

#[cfg(test)]
mod extreme_pixels {
    use super::*;
    #[test]
    fn aggregate_pixel_estimates_saturate() {
        let image = ContentPart::Image(crate::Image {
            width: Some(u32::MAX),
            height: Some(u32::MAX),
            ..crate::Image::from_bytes("image/png", b"x").unwrap()
        });
        let pictures = vec![image; 1000];
        assert_eq!(parts(&pictures), u64::MAX);
        let messages = [
            Message::user(pictures),
            Message::text(crate::Role::User, "more"),
        ];
        assert_eq!(
            estimate(
                &[SystemBlock {
                    text: "system".into(),
                    cache: false
                }],
                &messages,
                &[]
            ),
            u64::MAX
        );
    }
}

#[cfg(test)]
mod composition_tests {
    use super::*;
    #[test]
    fn composition_partitions_nested_images_and_rounds_each_text_once() {
        let system = [SystemBlock {
            text: "five!".into(),
            cache: false,
        }];
        let picture = ContentPart::Image(crate::Image::from_bytes("image/png", b"x").unwrap());
        let messages = [Message::user(vec![
            ContentPart::text("hello"),
            ContentPart::ToolResult {
                tool_use_id: "c".into(),
                parts: vec![picture, ContentPart::text("字")],
                is_error: false,
            },
        ])];
        let parts = composition(&system, &messages, &[]);
        assert_eq!(
            parts.iter().map(|p| p.tokens).sum::<u64>(),
            estimate(&system, &messages, &[])
        );
        assert_eq!(
            parts
                .iter()
                .map(|p| (p.name.to_string(), p.tokens))
                .collect::<Vec<_>>(),
            [("system", 2), ("tools", 0), ("chat", 3), ("pictures", 1600)]
                .map(|(name, count)| (name.to_string(), count))
        );
    }
}
