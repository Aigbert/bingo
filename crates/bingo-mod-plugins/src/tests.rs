use super::*;
use bingo_services::mcp::{Auth, ServerRow, Status};
use serde_json::json;
fn fixture() -> (Vec<CatalogEntry>, Vec<ServerRow>) {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/panel.json")).unwrap();
    (
        serde_json::from_value(v["plugins"].clone()).unwrap(),
        serde_json::from_value(v["servers"].clone()).unwrap(),
    )
}
#[test]
fn groups_keep_the_opener_first_and_cursor_on_its_name() {
    let (plugins, mut servers) = fixture();
    let groups = groups::groups(component::PluginsOpen::Plugins, &plugins, &servers, false);
    assert_eq!(
        groups.iter().map(|g| g.name).collect::<Vec<_>>(),
        [
            "plugins · yours",
            "plugins · this project",
            "plugins · built in",
            "MCP servers"
        ]
    );
    assert_eq!(groups[1].rows[0].tag, "waits for your yes");
    let mut cursor = Cursor {
        selected: "m:linear".into(),
        ..Default::default()
    };
    servers.reverse();
    let groups = groups::groups(component::PluginsOpen::Mcp, &plugins, &servers, false);
    assert_eq!(groups[0].name, "MCP servers");
    assert_eq!(
        groups::selected(&groups, &mut cursor).unwrap().label,
        "linear"
    );
}
#[test]
fn every_server_primary_action_is_a_command_and_reconnect_is_guarded() {
    let (_, mut servers) = fixture();
    let server = &mut servers[0];
    for (status, verb) in [
        (Status::Connected { tools: 3 }, "tools"),
        (Status::NeedsAuth { why: "401".into() }, "login"),
        (Status::Failed { why: "exit".into() }, "reconnect"),
        (Status::Disabled, "enable"),
    ] {
        server.status = status;
        assert_eq!(
            verbs::server(server, false, "enter"),
            verbs::Verb::Line(format!("/mcp {verb} github"))
        );
    }
    for status in [
        Status::Connecting,
        Status::SigningIn,
        Status::Disabled,
        Status::NeedsAuth { why: "401".into() },
    ] {
        server.status = status;
        assert!(matches!(
            verbs::server(server, false, "r"),
            verbs::Verb::Message(_)
        ));
    }
    server.auth = Some(Auth::SignedIn);
    assert_eq!(
        verbs::server(server, false, "o"),
        verbs::Verb::Line("/mcp logout github".into())
    );
}
#[test]
fn switches_use_the_wish_and_needed_plugins_are_refused() {
    let (plugins, _) = fixture();
    assert_eq!(
        verbs::plugin(&plugins[0], "space"),
        verbs::Verb::Line("/plugins disable my-picker".into())
    );
    assert_eq!(
        verbs::plugin(&plugins[6], "space"),
        verbs::Verb::Line("/plugins enable bingo.tools.bash".into())
    );
    assert!(matches!(
        verbs::plugin(&plugins[5], "space"),
        verbs::Verb::Message(_)
    ));
    assert_eq!(verbs::plugin(&plugins[3], "enter"), verbs::Verb::Trust);
    assert_eq!(
        verbs::plugin(&plugins[1], "r"),
        verbs::Verb::Line("/plugins reload".into())
    );
}
#[test]
fn full_requires_two_enters_and_moving_resets_confirmation() {
    let (plugins, _) = fixture();
    let p = &plugins[3];
    let mut choice = trust::Choice {
        selected: 2,
        confirm: false,
    };
    assert_eq!(trust::press(&mut choice, p, "enter"), None);
    assert!(choice.confirm);
    assert_eq!(
        trust::press(&mut choice, p, "enter"),
        Some(verbs::Verb::Line(
            "/plugins approve plugins:demo-lint full".into()
        ))
    );
    trust::press(&mut choice, p, "up");
    assert!(!choice.confirm);
    choice.selected = 3;
    assert!(matches!(
        trust::press(&mut choice, p, "enter"),
        Some(verbs::Verb::Message(_))
    ));
}
#[test]
fn cards_preserve_errors_and_name_trust_storage() {
    let (plugins, servers) = fixture();
    let error = cards::plugin_card(&plugins[1], &plugins, None)
        .into_iter()
        .find(|line| line.error)
        .unwrap();
    assert_eq!(error.text, plugins[1].meta["state"]["why"]);
    let card = cards::plugin_card(&plugins[3], &plugins, Some(&trust::Choice::default()));
    assert!(
        card.iter()
            .any(|line| line.text.contains("~/.bingo/settings.toml, never the repo"))
    );
    assert!(cards::server_card(&servers[0], false).iter().any(|line| {
        line.text
            .contains("always ask; an MCP tool is never trusted")
    }));
}
#[test]
fn handlers_retire_the_opening_and_return_only_command_lines() {
    let (plugins, servers) = fixture();
    let props = component::PluginsProps {
        open: component::PluginsOpen::Plugins,
        session: SessionId::from_raw("s"),
        plugins,
        action: Action {
            name: "surface:plugins".into(),
            args: json!({"handler":"a"}),
        },
    };
    let state = Shared::default();
    let handler = keys::Handler(state.clone());
    assert_eq!(
        handler.press("a", "space", &props, &servers).unwrap(),
        json!({"run":"/plugins disable my-picker"})
    );
    assert_eq!(
        handler.press("a", "esc", &props, &servers).unwrap(),
        json!({"close":true})
    );
    assert!(state.lock().unwrap().is_empty());
    state.lock().unwrap().insert(
        (props.session.clone(), "b".into()),
        Cursor {
            selected: "m:github".into(),
            ..Default::default()
        },
    );
    assert_eq!(
        handler.press("b", "enter", &props, &servers).unwrap(),
        json!({"run":"/mcp tools github"})
    );
    assert!(state.lock().unwrap().is_empty());
}

#[test]
fn removing_the_selected_row_drops_its_unconfirmed_trust_choice() {
    let (mut plugins, servers) = fixture();
    let mut cursor = Cursor {
        selected: "p:plugins:demo-lint".into(),
        trust: Some(trust::Choice {
            selected: 2,
            confirm: true,
        }),
        ..Default::default()
    };
    plugins.retain(|p| p.id != "plugins:demo-lint");
    let rows = groups::groups(component::PluginsOpen::Plugins, &plugins, &servers, false);
    groups::selected(&rows, &mut cursor);
    assert!(cursor.trust.is_none());
}

#[test]
fn approval_keeps_the_project_rows_identity() {
    let (mut plugins, servers) = fixture();
    plugins[3].id = "demo-lint".into();
    plugins[3].meta["state"] = json!({"kind":"on"});
    let mut cursor = Cursor {
        selected: "p:plugins:demo-lint".into(),
        ..Default::default()
    };
    let rows = groups::groups(component::PluginsOpen::Plugins, &plugins, &servers, false);
    assert_eq!(
        groups::selected(&rows, &mut cursor).unwrap().label,
        "demo-lint"
    );
}

#[test]
fn opening_links_never_wrap_other_commands_or_subcommands() {
    for name in ["plugins", "mcp"] {
        let matcher = opening::Opening(name).matcher();
        assert!(matcher.matches(catalogue::COMMAND_RUN, &json!({"command":name,"args":""})));
        assert!(!matcher.matches(
            catalogue::COMMAND_RUN,
            &json!({"command":name,"args":"disable local"})
        ));
        assert!(!matcher.matches(
            catalogue::COMMAND_RUN,
            &json!({"command":"thinking","args":"maximum"})
        ));
    }
}

/// The footer is read off the same table the keys are: a key it names does
/// what it says, and a key it leaves out only explains itself.
#[test]
fn the_footer_names_exactly_the_keys_that_act() {
    let (plugins, mut servers) = fixture();
    let mut tables = plugins.iter().map(verbs::plugin_keys).collect::<Vec<_>>();
    for status in [
        Status::Connected { tools: 3 },
        Status::NeedsAuth { why: "401".into() },
        Status::Failed { why: "exit".into() },
        Status::Disabled,
        Status::Connecting,
        Status::Waiting { changed: false },
    ] {
        servers[0].status = status;
        tables.push(verbs::server_keys(&servers[0], false));
        tables.push(verbs::server_keys(&servers[0], true));
    }
    for table in tables {
        let footer = draw::footer(&table, "close");
        for binding in &table {
            let named = binding.hint.is_some();
            let quiet = matches!(binding.verb, verbs::Verb::Message(_) | verbs::Verb::None);
            assert_eq!(named, !quiet, "{binding:?} in {footer}");
        }
    }
}

#[test]
fn a_waiting_server_starts_nothing_until_a_second_enter() {
    let (_, mut servers) = fixture();
    servers[0].status = Status::Waiting { changed: false };
    let footer = draw::footer(&verbs::server_keys(&servers[0], false), "close");
    assert_eq!(footer, " ↑↓ pick · ⏎ review · esc close");
    for key in ["space", " ", "r"] {
        assert!(matches!(
            verbs::server(&servers[0], false, key),
            verbs::Verb::Message(_)
        ));
    }
    assert_eq!(
        verbs::server(&servers[0], false, "enter"),
        verbs::Verb::Review
    );
    assert_eq!(
        verbs::server(&servers[0], true, "enter"),
        verbs::Verb::Line("/mcp approve github".into())
    );
    assert!(matches!(
        verbs::server(&servers[0], true, "space"),
        verbs::Verb::Message(_)
    ));
}

#[test]
fn approving_a_server_shows_what_it_runs_and_moving_forgets_it() {
    let (plugins, mut servers) = fixture();
    servers[0].status = Status::Waiting { changed: false };
    let props = component::PluginsProps {
        open: component::PluginsOpen::Mcp,
        session: SessionId::from_raw("s"),
        plugins,
        action: Action {
            name: "surface:plugins".into(),
            args: json!({"handler":"a"}),
        },
    };
    let handler = keys::Handler(Shared::default());
    let press = |key| handler.press("a", key, &props, &servers).unwrap();
    assert!(press("space")["message"].is_string());
    assert_eq!(press("enter"), json!({}));
    let card = cards::server_card(&servers[0], true);
    assert!(card.iter().any(|fact| fact.text.contains("local-fixture")));
    assert!(
        card.iter()
            .any(|fact| fact.error && fact.text.contains("⏎ again"))
    );
    assert_eq!(press("down"), json!({}));
    assert_eq!(press("up"), json!({}));
    assert_eq!(press("enter"), json!({}), "moving away forgot the review");
    assert_eq!(press("enter"), json!({"run":"/mcp approve github"}));
    assert_eq!(press("enter"), json!({}), "a yes starts a new review");
    assert_eq!(
        press("esc"),
        json!({}),
        "esc ends the review, not the panel"
    );
    assert_eq!(press("esc"), json!({"close":true}));
}
