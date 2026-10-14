//! Engine nodes return to the asking site's own typed drawing.
use bingo_sdk::{View, component::Props};
use ratatui::text::Line;
pub mod items;

pub fn draw(
    props: &Props,
    answer: Option<&View>,
    width: usize,
    own: impl Fn(&Props, usize) -> Vec<Line<'static>>,
) -> Vec<Line<'static>> {
    let Some(answer) = answer else {
        return own(props, width);
    };
    let invalid = std::cell::Cell::new(false);
    let engine = |component, value: &serde_json::Value, width| match Props::parse(
        component,
        value.clone(),
    ) {
        Ok(changed) if changed.component() == props.component() => own(&changed, width),
        _ => {
            invalid.set(true);
            own(props, width)
        }
    };
    let mut lines = crate::views::render_with(answer, width, &engine);
    if invalid.get() {
        lines.push(Line::from(ratatui::text::Span::styled(
            "invalid component props; showing the original drawing",
            crate::theme::dim(),
        )));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::component::InfoNoticeProps;
    use serde_json::json;
    fn own(props: &Props, _: usize) -> Vec<Line<'static>> {
        vec![Line::from(
            props.value()["text"].as_str().unwrap_or("own").to_owned(),
        )]
    }
    #[test]
    fn engine_uses_rewritten_props_inside_a_box() {
        let props = Props::InfoNotice(InfoNoticeProps {
            text: "original".into(),
            command: None,
        });
        assert_eq!(draw(&props, None, 80, own)[0].to_string(), "original");
        let answer: View = serde_json::from_value(json!({"kind":"Box","children":[
            {"kind":"Text","children":["before"]},
            {"kind":"Engine","component":"InfoNotice","props":{"text":"rewritten","command":null}}
        ]}))
        .unwrap();
        assert_eq!(
            draw(&props, Some(&answer), 80, own)
                .iter()
                .map(|line| line.to_string().trim_end().to_owned())
                .collect::<Vec<_>>(),
            ["before", "rewritten"]
        );
    }
    #[test]
    fn invalid_engine_props_draw_the_original_and_one_notice() {
        let props = Props::InfoNotice(InfoNoticeProps {
            text: "original".into(),
            command: None,
        });
        let answer = View::Engine {
            component: props.component(),
            props: json!({"text":42}),
        };
        let lines = draw(&props, Some(&answer), 80, own);
        assert_eq!(lines[0].to_string(), "original");
        assert_eq!(lines.len(), 2);
        assert!(lines[1].to_string().contains("invalid component props"));
    }
}

pub fn key(
    state: &bingo_sdk::SessionState,
    id: &str,
    props: &Props,
    width: u16,
    rows: u16,
) -> Option<crate::pulled::Key> {
    use bingo_sdk::render::{RenderRequest, SurfaceName, Viewport};
    let request = RenderRequest {
        component: props.component(),
        request_id: id.into(),
        surface: SurfaceName::Terminal,
        props: props.value(),
        viewport: Viewport {
            columns: width,
            rows,
        },
    };
    bingo_sdk::render::matched(&state.config)
        .asks(request.component, &request)
        .then(|| crate::pulled::Key::new(state, &request, width, rows))
}

pub fn band(
    state: &bingo_sdk::SessionState,
    ui: &crate::ui::Ui,
    frame: &mut crate::layout::Layout,
) -> Vec<Line<'static>> {
    use bingo_sdk::component::{AbovePromptProps, SiteScroll, SiteView};
    if ui.activity_hidden {
        return Vec::new();
    }
    let area = frame.area();
    let max_rows = area.height / 2;
    let props = Props::AbovePrompt(AbovePromptProps {
        has_survey: false,
        is_working: state.busy(),
        max_rows,
        body_columns: area.width,
        scroll: SiteScroll {
            offset: 0,
            body_rows: usize::from(max_rows),
        },
        view: SiteView { agent_id: None },
    });
    let Some(key) = key(state, "AbovePrompt", &props, area.width, max_rows) else {
        return Vec::new();
    };
    let mut lines = draw(
        &props,
        ui.placed.pulled.get(&key),
        usize::from(area.width),
        |_, _| Vec::new(),
    );
    frame.pulls.push(key);
    lines.truncate(usize::from(max_rows));
    lines
}

#[cfg(test)]
mod band_tests {
    use super::*;
    use crate::test_support::{draw_tree, drawn, scene, solo, state};
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use serde_json::json;

    #[test]
    fn above_prompt_button_dispatches_its_handler_from_the_drawn_band() {
        let mut state = state();
        state.config.kernel["render"] = json!({"AbovePrompt":[{}]});
        let tree = solo(&state);
        let (mut ui, now) = scene();
        draw_tree(80, 24, &tree, &mut ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component == bingo_sdk::render::Component::AbovePrompt)
            .unwrap()
            .clone();
        let action = bingo_sdk::Action {
            name: "band:ui".into(),
            args: json!({"handler":"hide","event":"ui.press","value":null}),
        };
        let answer: View = serde_json::from_value(
            json!({"kind":"Button","children":["Hide band"],"onPress":action}),
        )
        .unwrap();
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(key, answer);
        let backend = drawn(80, 24, &tree, &mut ui, now);
        let buffer = backend.buffer();
        let at = buffer
            .content
            .chunks(80)
            .enumerate()
            .find_map(|(row, cells)| {
                let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
                line.find("Hide band")
                    .map(|column| (column as u16, row as u16))
            })
            .expect("the TestBackend draws the mod's band button");
        let mut effects = crate::pointer::on_mouse(
            &mut ui,
            &tree,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: at.0,
                row: at.1,
                modifiers: KeyModifiers::NONE,
            },
            now,
        );
        effects.extend(crate::input::on_key(
            &mut ui,
            &tree,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            now,
        ));
        assert!(effects.iter().any(|effect| matches!(effect, crate::effect::Effect::Submit(bingo_sdk::Input::Action { action: fired, .. }) if fired == &action)),
            "the drawn band button must dispatch its handler; effects: {effects:?}");
    }
}
