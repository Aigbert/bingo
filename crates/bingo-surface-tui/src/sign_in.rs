//! A device-code sign-in docks as a sheet: the sealed `SignIn` site's answer,
//! or the surface's own drawing of it.
use crate::{clock::Now, layout::Layout, tree::Tree, ui::Ui, views};
use bingo_sdk::{
    InteractionKind, LoginFlow, View, pick,
    sheet::Sheet,
    view::{TextChild, TextMotion, TextStyle},
};
use ratatui::text::Line;

/// The device code is the one flow the dock draws; the others stay a card.
pub fn docked(tree: &Tree, ui: &Ui) -> bool {
    !ui.focus.showing()
        && tree.open_interaction().is_some_and(|(_, open)| {
            matches!(
                open.kind,
                InteractionKind::Login {
                    flow: LoginFlow::Device { .. },
                    ..
                }
            )
        })
}

pub fn prepare(
    tree: &Tree,
    ui: &Ui,
    frame: &mut Layout,
    cap: u16,
    now: Now,
) -> Option<Vec<Line<'static>>> {
    if !docked(tree, ui) {
        return None;
    }
    let (owner, interaction) = tree.open_interaction()?;
    let width = frame.area().width;
    let tools = crate::tool_rows::Looks::in_ui(ui);
    let site = crate::component::sealed::Site {
        state: owner,
        tools: &tools,
        ui,
        interaction,
        agent: None,
        width: usize::from(width),
        room: usize::from(cap),
        now,
    };
    if let Some(drawing) = site.draw(&mut frame.pulls) {
        let lines = drawing.rows.into_iter().map(|(line, _)| line).collect();
        return Some(frame.collect(views::Drawing {
            lines,
            next: drawing.next,
        }));
    }
    let view = sheet(&interaction.kind, width, cap)?;
    Some(frame.collect(views::marked(
        &view,
        usize::from(width),
        &views::Marks::at(now),
    )))
}

/// The engine's answer for the site, the one a mod's answer replaces.
pub fn sheet(kind: &InteractionKind, columns: u16, rows: u16) -> Option<View> {
    let InteractionKind::Login {
        provider,
        flow: LoginFlow::Device { url, code },
    } = kind
    else {
        return None;
    };
    Some(
        Sheet {
            head: format!("Sign in to {provider}"),
            fact: Some(pick::text("device code", "dim")),
            top: device(url, code),
            rows: vec![],
            selected: None,
            foot: vec![],
            pick: false,
        }
        .view(columns, rows, crate::theme::glyphs::set()),
    )
}

fn device(url: &str, code: &str) -> Vec<View> {
    let blank = || pick::text("", "fg");
    vec![
        pick::line([
            pick::text("open ", "dim"),
            underlined(url),
            pick::text(" and enter this code", "dim"),
        ]),
        blank(),
        pick::line([pick::text("  ", "fg"), pick::bold(code, "fg")]),
        blank(),
        pick::line([
            spinner(),
            pick::text("  waiting for you to approve in the browser", "muted"),
        ]),
    ]
}

fn underlined(text: &str) -> View {
    let mut view = pick::text(text, "code");
    if let View::Text { style, .. } = &mut view {
        style.underline = Some(true);
    }
    view
}

fn spinner() -> View {
    View::Text {
        children: vec![TextChild::String(crate::theme::glyphs::get("still").into())],
        style: TextStyle {
            color: Some("glow".into()),
            motion: Some(Box::new(TextMotion::Spin)),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_code() -> InteractionKind {
        InteractionKind::Login {
            provider: "anthropic".into(),
            flow: LoginFlow::Device {
                url: "console.anthropic.com/device".into(),
                code: "WDJB-MJHT".into(),
            },
        }
    }

    fn text(view: &View) -> Vec<String> {
        let drawn = views::marked(view, 60, &views::Marks::default());
        drawn
            .lines
            .iter()
            .map(|line| line.to_string().trim_end().to_owned())
            .collect()
    }

    #[test]
    fn the_device_code_sheet_reads_head_url_code_and_wait() {
        let rows = text(&sheet(&device_code(), 60, 12).unwrap());
        assert!(rows[0].starts_with("   Sign in to anthropic"), "{rows:?}");
        assert!(rows[0].ends_with("device code"), "{rows:?}");
        assert_eq!(
            rows[2],
            "   open console.anthropic.com/device and enter this code"
        );
        assert_eq!(rows[4], "     WDJB-MJHT");
        assert!(rows[6].ends_with("  waiting for you to approve in the browser"));
    }

    fn asked() -> (bingo_sdk::SessionState, Ui, crate::clock::Now) {
        use crate::test_support::*;
        let state = folded(vec![frame(
            1,
            opened(login(LoginFlow::Device {
                url: "console.anthropic.com/device".into(),
                code: "WDJB-MJHT".into(),
            })),
        )]);
        let (mut ui, now) = settled();
        ui.dialog.focus_on(state.interactions.first());
        (state, ui, now)
    }

    #[test]
    fn the_docked_sheet_replaces_the_card_and_esc_still_cancels() {
        use crate::test_support::*;
        let (state, mut ui, now) = asked();
        let drawn = draw_tree(80, 24, &solo(&state), &mut ui, now);
        assert!(drawn.contains("device code"), "{drawn}");
        assert!(!drawn.contains('╭'), "{drawn}");
        let effects = crate::input::on_key(
            &mut ui,
            &solo(&state),
            crossterm::event::KeyCode::Esc.into(),
            now,
        );
        assert!(
            matches!(
                effects.as_slice(),
                [crate::effect::Effect::StopInteraction {
                    answer: Some(bingo_sdk::Answer::Cancel),
                    ..
                }]
            ),
            "{effects:?}"
        );
    }

    #[test]
    fn a_mods_sign_in_answer_redraws_the_sheet() {
        use crate::test_support::*;
        let (mut state, mut ui, now) = asked();
        state.config.kernel["render"] = serde_json::json!({"SignIn":[{}]});
        draw_tree(80, 24, &solo(&state), &mut ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component == bingo_sdk::render::Component::SignIn)
            .unwrap()
            .clone();
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(
            key.clone(),
            View::column(vec![View::text("Sign in from a mod"), key.request.engine()]),
        );
        // The dock grows into the taller drawing over its own frames.
        let drawn = draw_tree(80, 24, &solo(&state), &mut ui, still(now));
        assert!(drawn.contains("Sign in from a mod"), "{drawn}");
        assert!(drawn.contains("WDJB-MJHT"), "{drawn}");
        assert!(!drawn.contains('╭'), "{drawn}");
    }

    #[test]
    fn a_browser_or_pasted_sign_in_is_not_the_sheet() {
        for flow in [LoginFlow::Paste, LoginFlow::Browser { url: "u".into() }] {
            let kind = InteractionKind::Login {
                provider: "anthropic".into(),
                flow,
            };
            assert!(sheet(&kind, 60, 12).is_none());
        }
    }
}
