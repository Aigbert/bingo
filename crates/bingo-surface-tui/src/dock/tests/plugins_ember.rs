use super::ember_tests::{fixture, show};
use crate::test_support::{
    ember::{Band, Capture, assert_regions},
    *,
};
use bingo_sdk::*;
use serde_json::json;

fn catalogue(switched: bool) -> Vec<CatalogEntry> {
    crate::test_support::ember::world::rows("plugins").into_iter().map(|row| {
        let name = row["n"].as_str().unwrap();
        let on = row["on"].as_bool().unwrap() && !(name == "tool-fs" && switched);
        let id = if row["kind"] == "mcp" { format!("mcp:{name}") } else { name.into() };
        CatalogEntry { id, label: name.into(), meta: json!({"from":if row["kind"] == "built in" { BUILT_IN } else { "user" },"description":row["d"],"state":if on { json!({"kind":"on"}) } else { json!({"kind":"off","why":"switched off"}) }}) }
    }).collect()
}

#[test]
fn m198_plugins_full_frames() {
    for (name, switched) in [("plugins", false), ("plugins-toggled", true)] {
        let capture = Capture::load(name);
        capture.render(|| {
            let (mut state, mut ui) = fixture(&capture);
            let props = component::PluginsProps {
                open: component::PluginsOpen::Plugins,
                session: state.summary.id.clone(),
                plugins: catalogue(switched),
                action: Action {
                    name: "surface:plugins".into(),
                    args: json!({}),
                },
            };
            let servers: Vec<bingo_services::mcp::ServerRow> = serde_json::from_value(json!([{
                "name":"github", "transport":{"type":"http","url":"https://example.test"},
                "status":{"kind":"needsAuth","why":"token expired"}
            }, {"name":"linear","transport":{"type":"http","url":"https://example.test"},"status":{"kind":"disabled"}}]))
            .unwrap();
            let mut cursor = bingo_mod_plugins::Cursor {
                selected: if switched { "p:tool-fs" } else { "m:github" }.into(),
                ..Default::default()
            };
            let view = bingo_mod_plugins::draw::tree(
                &props,
                &servers,
                &mut cursor,
                "plugins",
                100,
                19,
                Default::default(),
            );
            show(&mut state, &mut ui, "plugins", view);
            if switched {
                ui.notify(
                    Level::Info,
                    "tool-fs off · from the next turn",
                    capture.now().instant - std::time::Duration::from_millis(300),
                );
            }
            let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
            assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
        });
    }
}
