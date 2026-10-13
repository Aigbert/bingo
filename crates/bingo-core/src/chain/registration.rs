//! Registration may only subtract from the grant supplied by the person.
use bingo_sdk::*;

pub(crate) fn reduced(before: &PluginGrant, after: &PluginGrant) -> bool {
    rank(after.tier) <= rank(before.tier) && after.uses.is_subset(&before.uses)
}

fn rank(tier: Tier) -> u8 {
    match tier {
        Tier::Tighten => 0,
        Tier::Rewrite => 1,
        Tier::Full => 2,
    }
}

pub(crate) fn values(before: &serde_json::Value, after: &serde_json::Value) -> bool {
    match (
        serde_json::from_value::<PluginGrant>(before.clone()),
        serde_json::from_value::<PluginDecision>(after.clone()),
    ) {
        (Ok(before), Ok(PluginDecision::Granted(after))) => reduced(&before, &after),
        (Ok(_), Ok(PluginDecision::Denied { reason })) => !reason.trim().is_empty(),
        _ => false,
    }
}

pub(crate) async fn register(
    input: PluginRegistration,
    links: &[super::Link],
    cx: &HookContext,
) -> Result<PluginGrant, String> {
    let name = input.name.clone();
    let report = super::Report::default();
    let result = super::json::dispatch(
        catalogue::PLUGIN_REGISTER,
        links,
        cx,
        input,
        Next::new(|input: PluginRegistration| {
            Box::pin(async move { Ok(PluginDecision::Granted(input.execution.grant)) })
        }),
        report.clone(),
    )
    .await;
    for item in report.take() {
        if let ItemBody::Notice { level, code, text } = item {
            let _ = cx.host.notice(level, &code, &text).await;
        }
    }
    match result {
        Ok(PluginDecision::Granted(grant)) => Ok(grant),
        Ok(PluginDecision::Denied { reason }) => Err(format!("{name}: {reason}")),
        Err(error) => Err(format!("{name}: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_can_only_lower_tier_and_drop_nouns() {
        let tiers = [Tier::Tighten, Tier::Rewrite, Tier::Full];
        for (i, before) in tiers.iter().enumerate() {
            for (j, after) in tiers.iter().enumerate() {
                let before = PluginGrant {
                    tier: *before,
                    uses: ["state".into(), "model".into()].into(),
                };
                let mut after = PluginGrant {
                    tier: *after,
                    uses: ["state".into()].into(),
                };
                assert_eq!(reduced(&before, &after), j <= i);
                after.uses.insert("process".into());
                assert!(!reduced(&before, &after));
            }
        }
    }
}
