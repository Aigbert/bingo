//! Both plugins must interpret the same roster identically until M122.
#[test]
fn agents_and_rooms_compare_one_table_of_names_the_same_way() {
    for (a, b, expected) in [
        ("Reviewer", "reviewer", true), ("Parent", "PARENT", true),
        ("Éclair", "éclair", true), ("Σ", "σ", true),
        ("İ", "i\u{307}", true), ("Straße", "STRASSE", false),
        ("reviewer", "reviewers", false), ("a", " a", false),
        ("#DESIGN", "#design", true),
    ] {
        assert_eq!(bingo_rooms::same_name(a, b), expected, "{a} / {b}");
        assert_eq!(bingo_agents::same_name(a, b), expected, "{a} / {b}");
    }
}
