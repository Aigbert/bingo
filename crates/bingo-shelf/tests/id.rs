use bingo_shelf::id::{Named, mint, resolve};
const LEN: usize = 8;

#[test]
fn a_minted_id_is_short_lowercase_and_alphanumeric() {
    let ids: Vec<String> = (0..100).map(|_| mint()).collect();
    for id in &ids {
        assert_eq!(id.chars().count(), LEN, "{id}");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "{id}"
        );
    }
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "minted ids collided");
}

#[test]
fn a_unique_prefix_names_one_entry() {
    let ids = ["ab12cd34", "ab99zz00", "ffffffff"];
    assert_eq!(resolve(&ids, "ff", |id| *id), Named::One(&"ffffffff"));
    assert_eq!(resolve(&ids, "ab12cd34", |id| *id), Named::One(&"ab12cd34"));
    assert_eq!(resolve(&ids, "zz", |id| *id), Named::Unknown);
    assert_eq!(resolve(&ids, "", |id| *id), Named::Unknown);
    assert_eq!(
        resolve(&ids, "ab", |id| *id),
        Named::Ambiguous(vec!["ab12cd34", "ab99zz00"])
    );
}

#[test]
fn an_exact_id_wins_over_the_ids_it_prefixes() {
    assert_eq!(
        resolve(&["ab12", "ab12cd34"], "ab12", |id| *id),
        Named::One(&"ab12")
    );
}
