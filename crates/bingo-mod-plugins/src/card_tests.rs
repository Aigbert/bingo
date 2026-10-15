use super::*;
use bingo_services::mcp::{ServerRow, Status, Transport};
fn server() -> ServerRow {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/panel.json")).unwrap();
    let servers: Vec<ServerRow> = serde_json::from_value(v["servers"].clone()).unwrap();
    servers[0].clone()
}
fn said(card: &[cards::Fact]) -> Vec<&str> {
    card.iter().map(|fact| fact.text.as_str()).collect()
}
/// A repository's server that sets a variable which loads code shows that
/// value, marked, on its review; every other value stays hidden.
#[test]
fn a_review_shows_an_environment_value_that_loads_code() {
    let mut repo = server();
    repo.status = Status::Waiting { changed: false };
    repo.transport = Transport::Stdio {
        command: "npx".into(),
        args: vec!["-y".into(), "files".into()],
        env: [
            ("NODE_OPTIONS".into(), "--require ./evil.js".into()),
            ("TOKEN".into(), "s3cret".into()),
        ]
        .into(),
        cwd: None,
    }
    .reviewed();
    let card = cards::server_card(&repo, true);
    let said = said(&card);
    assert!(
        said.contains(&" env NODE_OPTIONS=--require ./evil.js  (loads code)"),
        "{said:?}"
    );
    assert!(said.contains(&" env TOKEN (values hidden)"), "{said:?}");
    assert!(!said.concat().contains("s3cret"), "{said:?}");
}
/// A program has no sign-in: what it is handed is its environment.
#[test]
fn a_stdio_server_names_its_environment_not_a_sign_in() {
    let mut local = server();
    local.sign_in_from = Some("NODE_OPTIONS, TOKEN from settings".into());
    let card = cards::server_card(&local, false);
    let said = said(&card);
    assert!(
        !said.iter().any(|text| text.contains("sign-in")),
        "{said:?}"
    );
    assert!(
        said.contains(&" env      NODE_OPTIONS, TOKEN from settings"),
        "{said:?}"
    );
    local.transport = Transport::Http {
        url: "https://mcp.example.com/mcp".into(),
        headers: Default::default(),
    };
    let card = cards::server_card(&local, false);
    assert!(said_any(
        &card,
        " sign-in  NODE_OPTIONS, TOKEN from settings"
    ));
}
fn said_any(card: &[cards::Fact], text: &str) -> bool {
    card.iter().any(|fact| fact.text == text)
}
/// A wrapped value continues under its value, a wrapped command under the
/// command, and a word wider than the line is split rather than lost.
#[test]
fn a_wrapped_line_continues_under_its_value() {
    assert_eq!(
        cards::hanging(" trust    full · also allow a call without asking", 30, 10),
        " trust    full · also allow a\n          call without asking"
    );
    assert_eq!(
        cards::hanging("   npx -y @acme/github-mcp --repo=.", 20, 3),
        "   npx -y\n   @acme/github-mcp\n   --repo=."
    );
    assert_eq!(
        cards::hanging(" x abcdefghij", 6, 2),
        " x\n  abcd\n  efgh\n  ij"
    );
    assert_eq!(cards::hanging(" short", 30, 10), " short");
}
