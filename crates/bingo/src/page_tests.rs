use super::*;
use bingo_sdk::{Contribution, Page, Pages, Registrar};

#[test]
fn every_registered_page_has_a_short_description_and_body() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::rooted(home.path());
    let plugins = plugins(Composition::default(), &platforms(&env), home.path()).unwrap();
    let mut expected = std::collections::BTreeMap::from([
        ("bingo.agents", "agents"),
        ("bingo.surface.channels", "channels"),
        ("bingo.checkpoints", "rewind"),
        ("bingo.context", "memory"),
        ("bingo.experience", "experience"),
        ("bingo.hooks.shell", "hooks"),
        ("bingo.mcp", "mcp"),
        ("bingo.mods", "mods"),
        ("bingo.permissions", "permissions"),
        ("bingo.persona", "persona"),
        ("bingo.acp", "acp"),
        ("bingo.rooms", "rooms"),
        ("bingo.schedule", "schedule"),
        ("bingo.skills", "skills"),
        ("bingo.surface.tui", "tui"),
        ("bingo.tasks", "tasks"),
    ]);
    for plugin in plugins {
        let id = plugin.manifest().id;
        let service = Pages::key(id);
        let key = service.name();
        if !plugin
            .manifest()
            .provides
            .iter()
            .any(|value| *value == format!("service:{key}"))
        {
            assert!(!expected.contains_key(id), "{id} lost its page");
            continue;
        }
        let mut registrar = Registrar::new(id, json!({}), env.clone(), Default::default());
        plugin.register(&mut registrar).unwrap();
        let contributions = registrar.into_contributions();
        let pages = contributions
            .iter()
            .find_map(|contribution| match contribution {
                Contribution::Service {
                    key: actual, value, ..
                } if actual == key => value.downcast_ref::<Arc<Pages>>(),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{id} did not register {key}"));
        assert!(!pages.0.is_empty(), "{id}");
        if let Some(name) = expected.remove(id) {
            assert_eq!(pages.0[0].name, name, "{id}");
        }
        for page in pages.0 {
            shape(id, page);
        }
    }
    assert!(expected.is_empty(), "missing plugins: {expected:?}");
}

fn shape(id: &str, page: &Page) {
    assert!(!page.description.contains('\n'), "{id}: {}", page.name);
    assert!(
        page.description.chars().count() < 250,
        "{id}: {}",
        page.name
    );
    assert!(!page.body.is_empty(), "{id}: {}", page.name);
    assert!(page.body.lines().count() < 200, "{id}: {}", page.name);
}
