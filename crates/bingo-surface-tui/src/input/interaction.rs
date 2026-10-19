use super::*;

pub(crate) fn interaction_keys(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    if keys::look(key) == Some(keys::Look::Detail) {
        super::open_detail(ui, tree.viewed(), now);
        return Vec::new();
    }
    if guarded(interaction, now) {
        return Vec::new();
    }
    let ask = crate::ask::open(tree, ui);
    if key.code == KeyCode::Esc {
        if ask.is_some() && crate::ask::keys::back(ui) {
            return Vec::new();
        }
        return escape(ui, tree, now);
    }
    if ui.dialog.answered {
        return Vec::new();
    }
    if let Some((interaction, ask)) = ask {
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
fn guarded(interaction: &bingo_sdk::Interaction, now: Now) -> bool {
    match interaction.kind {
        bingo_sdk::InteractionKind::Login { .. } => interaction
            .guard_until
            .is_some_and(|until| now.wall < until),
        _ => !crate::ask::guard::open(interaction.opened_at, now.wall),
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

/// Leaving the card that is asking: the kernel's own cancel or denial, which
/// the dialog knows and this does not.
pub(crate) fn cancel(ui: &mut Ui, tree: &Tree) -> Vec<Effect> {
    let Some((owner, interaction)) = tree.open_interaction() else {
        return Vec::new();
    };
    if ui.dialog.aside {
        ui.dialog.aside = false;
        return Vec::new();
    }
    if matches!(
        interaction.kind,
        bingo_sdk::InteractionKind::Question(_) | bingo_sdk::InteractionKind::Form { .. }
    ) && interaction.answers.contains(&bingo_sdk::AnswerSpec::Cancel)
    {
        return skip(ui, interaction);
    }
    ui.dialog.words = None;
    ui.dialog.changing = None;
    ui.dialog.aside = false;
    ui.dialog.answered = true;
    ui.stop_asked = owner.turn.as_ref().map(|turn| turn.id.clone());
    vec![Effect::StopInteraction {
        session: owner.summary.id.clone(),
        interaction: interaction.id.clone(),
        answer: crate::dialog::cancel(interaction),
    }]
}

fn skip(ui: &mut Ui, interaction: &bingo_sdk::Interaction) -> Vec<Effect> {
    if ui.dialog.answered || !interaction.answers.contains(&bingo_sdk::AnswerSpec::Cancel) {
        return Vec::new();
    }
    ui.dialog.words = None;
    ui.dialog.changing = None;
    ui.dialog.answered = true;
    vec![Effect::Answer {
        interaction: interaction.id.clone(),
        answer: bingo_sdk::Answer::Cancel,
        activation: bingo_sdk::Activation::Keyboard,
    }]
}
