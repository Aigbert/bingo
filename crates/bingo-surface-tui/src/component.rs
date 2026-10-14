//! Engine nodes return to the asking site's own typed drawing.
use bingo_sdk::{View, component::Props};
use ratatui::text::Line;

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
