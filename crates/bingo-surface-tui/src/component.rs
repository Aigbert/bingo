//! Engine nodes return to the asking site's own typed drawing.
use bingo_sdk::{View, component::Props};
#[cfg(test)]
use ratatui::text::Line;
pub mod band;
pub mod closing;
mod facts;
pub mod items;
pub mod live;
pub mod pane;
pub mod question;
mod retry;
pub mod sealed;
pub mod welcome;

#[cfg(test)]
pub fn draw(
    props: &Props,
    answer: Option<&View>,
    width: usize,
    own: impl Fn(&Props, usize) -> Vec<Line<'static>>,
) -> Vec<Line<'static>> {
    draw_marked(props, answer, width, &crate::views::Marks::default(), own).lines
}

pub fn draw_marked<R: Into<crate::views::Drawing>>(
    props: &Props,
    answer: Option<&View>,
    width: usize,
    marks: &crate::views::Marks,
    own: impl Fn(&Props, usize) -> R,
) -> crate::views::Drawing {
    let Some(answer) = answer else {
        return own(props, width).into();
    };
    let invalid = std::cell::Cell::new(false);
    let engine = |component, value: &serde_json::Value, width| match Props::parse(
        component,
        value.clone(),
    ) {
        Ok(changed) if changed.component() == props.component() => own(&changed, width).into(),
        _ => {
            invalid.set(true);
            own(props, width).into()
        }
    };
    let mut lines = crate::views::marked_with(answer, width, marks, &engine);
    if invalid.get() {
        lines.lines.extend(crate::views::render(
            &View::Text {
                children: vec![bingo_sdk::view::TextChild::String(
                    "invalid component props; showing the original drawing".into(),
                )],
                style: bingo_sdk::view::TextStyle {
                    dim_color: Some(true),
                    ..Default::default()
                },
            },
            width,
        ));
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
    let matched = bingo_sdk::render::matched(&state.config);
    if !matched.0.contains_key(&props.component()) {
        return None;
    }
    let request = RenderRequest {
        component: props.component(),
        request_id: id.into(),
        surface: SurfaceName::Terminal,
        props: props.value(),
        viewport: Viewport {
            glyphs: Default::default(),
            place: None,
            columns: width,
            rows,
        },
    };
    matched
        .asks(request.component, &request)
        .then(|| crate::pulled::Key::new(state, &request, width, rows))
}

#[cfg(test)]
mod band_tests {
    use super::*;
    use crate::test_support::{draw_tree, drawn, scene, solo, state};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
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
        let effects = crate::pointer::on_mouse(
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
        assert!(effects.iter().any(|effect| matches!(effect, crate::effect::Effect::Submit(bingo_sdk::Input::Action { action: fired, .. }) if fired == &action)),
            "the drawn band button must dispatch its handler; effects: {effects:?}");
    }
}

#[cfg(test)]
mod sites_tests;

pub fn resolve<R: Into<crate::views::Drawing>>(
    state: &bingo_sdk::SessionState,
    memo: Option<&crate::pulled::Memo>,
    id: &str,
    props: &Props,
    width: usize,
    marks: &crate::views::Marks,
    own: impl Fn(&Props, usize) -> R,
) -> crate::views::Drawing {
    let key = key(state, id, props, width as u16, 0);
    let answer = key
        .as_ref()
        .and_then(|key| memo.and_then(|memo| memo.display(key)));
    draw_marked(props, answer, width, marks, own)
}

pub struct Site<'a> {
    pub state: &'a bingo_sdk::SessionState,
    pub memo: &'a crate::pulled::Memo,
    pub width: usize,
    pub pulls: &'a mut Vec<crate::pulled::Key>,
    pub now: crate::clock::Now,
}
impl Site<'_> {
    pub fn draw<R: Into<crate::views::Drawing>>(
        &mut self,
        id: &str,
        props: &Props,
        own: impl Fn(&Props, usize) -> R,
    ) -> crate::views::Drawing {
        let Some(key) = key(self.state, id, props, self.width as u16, 0) else {
            return own(props, self.width).into();
        };
        let answer = self.memo.display(&key);
        self.pulls.push(key);
        draw_marked(
            props,
            answer,
            self.width,
            &crate::views::Marks::at(self.now),
            own,
        )
    }
}

pub fn has_engine(view: &View, component: bingo_sdk::render::Component) -> bool {
    match view {
        View::Engine {
            component: found, ..
        } => *found == component,
        View::Box { children, .. } => children.iter().any(|child| has_engine(child, component)),
        View::Panel { child, .. } => has_engine(child, component),
        _ => false,
    }
}

#[cfg(test)]
mod model_picker_tests;
