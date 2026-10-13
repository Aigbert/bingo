//! Registration may only subtract from the grant supplied by the person.
use bingo_sdk::*;

pub(crate) fn reduced(before: &PluginGrant, after: &PluginGrant) -> bool {
    let _ = (before, after);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_can_only_lower_tier_and_drop_nouns() {
        let tiers = [Tier::Tighten, Tier::Rewrite, Tier::Full];
        for (i, before) in tiers.iter().enumerate() {
            for (j, after) in tiers.iter().enumerate() {
                let before = PluginGrant { tier: *before, uses: ["state".into(), "model".into()].into() };
                let mut after = PluginGrant { tier: *after, uses: ["state".into()].into() };
                assert_eq!(reduced(&before, &after), j <= i);
                after.uses.insert("process".into());
                assert!(!reduced(&before, &after));
            }
        }
    }
}
