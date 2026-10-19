use super::*;
use crate::test_support::*;

fn file(path: &str, change: Option<Change>) -> Entry {
    Entry::File {
        path: path.into(),
        change,
    }
}

fn agent(name: &str, state: Option<Lifecycle>) -> Entry {
    Entry::Agent {
        name: name.into(),
        ink: "crew1",
        state,
    }
}

/// The prototype's files, in its order.
fn files() -> Vec<Entry> {
    vec![
        file(
            "crates/bingo-core/src/session/queue.rs",
            Some(Change::Modified),
        ),
        file("crates/bingo-core/tests/queue_steer.rs", Some(Change::New)),
        file("docs/adr/0097-ember-tui.md", Some(Change::New)),
        file("crates/bingo-core/src/session/mod.rs", None),
        file("Cargo.toml", None),
    ]
}

fn texts(offers: &[Offer]) -> Vec<&str> {
    offers.iter().map(|offer| offer.entry.text()).collect()
}

fn drawn(query: &str, pool: &[Entry], selected: usize) -> String {
    draw(
        query,
        &ranked(query, pool),
        selected,
        (100, 20),
        GlyphSet::Unicode,
    )
    .fold()
}

#[test]
fn the_nearer_run_ranks_first_and_ties_keep_the_pools_order() {
    let offers = ranked("queue", &files());
    assert_eq!(
        texts(&offers),
        [
            "crates/bingo-core/tests/queue_steer.rs",
            "crates/bingo-core/src/session/queue.rs",
        ]
    );
    let pool = [vec![agent("reviewer", None)], files()].concat();
    assert_eq!(texts(&ranked("", &pool))[0], "reviewer");
}

#[test]
fn forty_matches_are_kept_at_most() {
    let pool = (0..60)
        .map(|at| file(&format!("src/f{at}.rs"), None))
        .collect::<Vec<_>>();
    assert_eq!(ranked("f", &pool).len(), KEPT);
}

#[test]
fn six_rows_show_with_no_count_then_a_blank() {
    let pool = (0..10)
        .map(|at| file(&format!("src/f{at}.rs"), None))
        .collect::<Vec<_>>();
    let text = drawn("", &pool, 0);
    assert!(text.contains("src/f5.rs"), "{text}");
    assert!(!text.contains("src/f6.rs"), "{text}");
    assert!(!text.contains(" of 10"), "{text}");
}

#[test]
fn a_file_says_what_git_says_of_it() {
    let text = drawn("queue", &files(), 0);
    assert!(
        text.contains("queue_steer.rs") && text.contains("new"),
        "{text}"
    );
    assert!(text.contains("modified"), "{text}");
    let unread = drawn("Cargo", &[file("Cargo.toml", None)], 0);
    assert!(
        !unread.contains("modified") && !unread.contains("new"),
        "{unread}"
    );
}

#[test]
fn a_teammate_says_what_it_is_doing_and_all_says_who_it_reaches() {
    let pool = [agent("reviewer", Some(Lifecycle::Working)), Entry::Everyone];
    let text = drawn("", &pool, 0);
    assert!(text.contains("agent · working"), "{text}");
    assert!(text.contains("everyone seated"), "{text}");
    let ascii = draw("", &ranked("", &pool), 0, (100, 20), GlyphSet::Ascii).fold();
    assert!(ascii.contains("agent - working"), "{ascii}");
}

#[test]
fn nothing_matching_says_so() {
    assert!(drawn("zz", &files(), 0).contains("nothing matches @zz"));
}

/// Each styled run of a line, with its ink.
fn runs(view: &View) -> Vec<(String, Option<String>)> {
    let View::Text { children, .. } = view else {
        panic!("a line of text")
    };
    children
        .iter()
        .map(|child| match child {
            bingo_sdk::view::TextChild::Node(node) => match node.as_ref() {
                View::Text { style, .. } => (node.fold(), style.color.clone()),
                other => panic!("text, not {other:?}"),
            },
            other => panic!("a node, not {other:?}"),
        })
        .collect()
}

#[test]
fn a_files_folders_are_dim_unless_matched() {
    let ink = |text: &str, color: &str| (text.to_owned(), Some(color.to_owned()));
    assert_eq!(
        runs(&super::file("src/queue.rs", &[1, 4], false)),
        [
            ink("s", "dim"),
            ink("r", "glow"),
            ink("c/", "dim"),
            ink("q", "glow"),
            ink("ueue.rs", "muted"),
        ]
    );
    assert_eq!(runs(&super::file("a.rs", &[], true)), [ink("a.rs", "fg")]);
}

#[test]
fn the_list_is_up_only_while_a_mention_is_typed_and_esc_closes_it() {
    let state = state();
    let (mut ui, now) = scene();
    write(&mut ui, &state, "look at @que", now);
    assert_eq!(query(&ui), Some("que"));
    ui.menu.dismissed = true;
    assert_eq!(query(&ui), None);
    write(&mut ui, &state, "u", now);
    assert_eq!(query(&ui), Some("queu"), "typing opens it again");
    write(&mut ui, &state, " ", now);
    assert_eq!(query(&ui), None, "a space finishes the mention");
}
