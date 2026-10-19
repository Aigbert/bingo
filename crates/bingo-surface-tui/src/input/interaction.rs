use super::*;

pub(crate) fn interaction_keys(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    if keys::look(key) == Some(keys::Look::Detail) {
        super::open_detail(ui, tree.viewed(), now);
        return Vec::new();
    }
    if guarded(ui, interaction, now) {
        return Vec::new();
    }
    if key.code == KeyCode::Esc && !crate::ask::question::writing(ui, interaction) {
        return escape(ui, tree, now);
    }
    if ui.dialog.answered {
        return Vec::new();
    }
    if let Some((interaction, ask)) = crate::ask::open(tree, ui) {
        return crate::ask::keys::on_key(ui, interaction, &ask, key, now);
    }
    if crate::ask::waiting(ui, owner, interaction) {
        return vec![Effect::AwaitSiteKey(key)];
    }
    if let Some(action) = crate::ask::handler(ui, interaction, key) {
        return vec![Effect::SiteKey { action, key }];
    }
    ui.dialog.on_key(interaction, key, now)
}

/// An ask takes no key, not even esc, until it has been on screen long
/// enough to be read; a sign-in keeps the kernel's own guard.
fn guarded(ui: &Ui, interaction: &bingo_sdk::Interaction, now: Now) -> bool {
    match interaction.kind {
        bingo_sdk::InteractionKind::Login { .. } => interaction
            .guard_until
            .is_some_and(|until| now.wall < until),
        _ => !crate::ask::guard::open(crate::ask::guard::armed(ui, interaction, now), now.wall),
    }
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

/// Leaving the ask: first the words typed into it, then the ask itself with
/// the kernel's own cancel or denial, which the dialog knows and this does not.
pub(crate) fn cancel(ui: &mut Ui, tree: &Tree, now: Now) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    if ui.dialog.aside {
        ui.dialog.aside = false;
        return Vec::new();
    }
    if guarded(ui, interaction, now) {
        return Vec::new();
    }
    // Words being typed into the ask are the first thing esc leaves.
    if crate::ask::question::writing(ui, interaction) {
        return interaction_keys(
            ui,
            tree,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            now,
        );
    }
    if crate::ask::keys::back(ui) {
        return Vec::new();
    }
    ui.dialog.words = None;
    ui.dialog.aside = false;
    ui.dialog.answered = true;
    ui.stop_asked = owner.turn.as_ref().map(|turn| turn.id.clone());
    vec![Effect::StopInteraction {
        session: owner.summary.id.clone(),
        interaction: interaction.id.clone(),
        answer: crate::dialog::cancel(interaction),
    }]
}
