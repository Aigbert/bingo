use super::*;
use crate::dollar::nouns::{CALLS, NOUNS};
use crate::dollar::table::ROWS;
use bingo_sdk::render::Component;
use bingo_sdk::{CatalogEntry, CatalogKind};

pub(crate) fn kernel_events() -> Catalog {
    let document = bingo_sdk::catalogue::document();
    Catalog {
        kind: CatalogKind::Events,
        entries: document["events"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(id, meta)| CatalogEntry {
                id: id.clone(),
                label: id.clone(),
                meta: meta.clone(),
            })
            .collect(),
    }
}

#[test]
fn the_reference_names_every_event_method_element_and_site_with_its_doc() {
    let text = reference(&kernel_events());
    for entry in bingo_sdk::catalogue::ALL {
        let doc = entry
            .description()
            .unwrap_or_else(|| panic!("{} has no doc", entry.name));
        assert!(text.contains(&format!("### `{}`\n\n{}", entry.name, one_line(&doc))));
    }
    for row in ROWS {
        assert!(
            text.contains(&one_line(row.doc)),
            "{}.{}",
            row.noun,
            row.method
        );
    }
    for call in CALLS {
        assert!(text.contains(&format!("`$.{}.{}`", call.noun, call.signature)));
    }
    for noun in NOUNS {
        assert!(text.contains(&format!(
            "### `$.{}`\n\n{}\n\n```ts\n{}\n```",
            noun.name, noun.doc, noun.example
        )));
    }
    for element in bingo_sdk::view::table::ELEMENTS {
        assert!(text.contains(&format!("### `{}`", element.kind)));
    }
    for component in Component::ALL {
        assert!(text.contains(component.summary()), "{component:?}");
    }
}

#[test]
fn site_props_and_method_inputs_are_listed_by_name() {
    let text = reference(&kernel_events());
    assert!(
        text.contains("### `Pane`") && text.contains("- `isFocused`: `boolean`"),
        "{text}"
    );
    assert!(text.contains("- `$.fs.read({"), "{text}");
    assert!(text.contains("- `$.clock.now()`"), "{text}");
    assert!(text.contains("Match on: `tool`."));
}
