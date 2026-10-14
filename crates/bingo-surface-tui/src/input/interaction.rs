use super::*;

pub(crate) fn interaction_keys(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    if key.code == KeyCode::Esc {
        return escape(ui, tree, now);
    }
    if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(item) = &interaction.item {
            ui.focus
                .show(Open::Pager(pager::Pager::open(item.clone())), now.instant);
        } else {
            ui.dialog.expanded = !ui.dialog.expanded;
        }
        return Vec::new();
    }
    if ui.dialog.answered
        || interaction
            .guard_until
            .is_some_and(|until| now.wall < until)
    {
        return Vec::new();
    }
    if crate::ask::waiting(ui, owner, interaction) {
        return vec![Effect::AwaitSiteKey(key)];
    }
    if let Some(action) = crate::ask::handler(ui, interaction, key) {
        return vec![Effect::SiteKey { action, key }];
    }
    interaction_fallback(ui, tree, owner, interaction, key, now)
}

fn interaction_fallback(
    ui: &mut Ui,
    tree: &Tree,
    owner: &SessionState,
    interaction: &bingo_sdk::Interaction,
    key: KeyEvent,
    now: Now,
) -> Vec<Effect> {
    if ui.dialog.words.is_none() && crate::ask::aside::printable(key, interaction) {
        ui.dialog.aside = true;
        return composer_keys(ui, tree, key, now);
    }
    if interaction
        .guard_until
        .is_none_or(|until| now.wall >= until)
        && crate::ask::begin_change(ui, owner, interaction, key)
    {
        return Vec::new();
    }
    ui.dialog.on_key(interaction, key, now)
}

pub(crate) fn interaction_unhandled(
    ui: &mut Ui,
    tree: &Tree,
    key: KeyEvent,
    now: Now,
) -> Vec<Effect> {
    if matches!(key.code, KeyCode::Char(_)) && (key.modifiers - KeyModifiers::SHIFT).is_empty() {
        ui.dialog.aside = true;
        return composer_keys(ui, tree, key, now);
    }
    Vec::new()
}

/// Leaving the card that is asking: the kernel's own cancel or denial, which
/// the dialog knows and this does not.
pub(crate) fn cancel(ui: &mut Ui, tree: &Tree, now: Now) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    ui.dialog.words = None;
    ui.dialog.changing = None;
    ui.dialog.aside = false;
    let effects = ui.dialog.on_key(
        interaction,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        now,
    );
    ui.stop_asked = owner.turn.as_ref().map(|turn| turn.id.clone());
    effects
        .into_iter()
        .map(|effect| match effect {
            Effect::Answer {
                interaction,
                answer,
                ..
            } => Effect::StopInteraction {
                session: owner.summary.id.clone(),
                interaction,
                answer,
            },
            other => other,
        })
        .collect()
}
