use super::*;

fn rank(external: bool, placement: bingo_sdk::plugin::Placement) -> u8 {
    match (external, placement) {
        (true, _) => 0,
        (false, bingo_sdk::plugin::Placement::Outer) => 1,
        (false, bingo_sdk::plugin::Placement::Builtin) => 2,
    }
}

pub fn ordered(mut links: Vec<Link>) -> Vec<Link> {
    links.sort_by_key(|link| rank(link.external.is_some(), link.placement));
    links
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_then_outer_then_builtin_preserves_declaration_order() {
        let mut rows = [
            (false, bingo_sdk::plugin::Placement::Builtin, 0),
            (true, bingo_sdk::plugin::Placement::Builtin, 1),
            (false, bingo_sdk::plugin::Placement::Outer, 2),
            (false, bingo_sdk::plugin::Placement::Builtin, 3),
            (true, bingo_sdk::plugin::Placement::Outer, 4),
        ];
        rows.sort_by_key(|(external, placement, _)| rank(*external, *placement));
        assert_eq!(rows.map(|(_, _, id)| id), [1, 4, 2, 0, 3]);
    }
}
