//! Engine nodes return to the asking site's own typed drawing.
use bingo_sdk::{View, component::Props};
use ratatui::text::Line;

pub fn draw(
    props: &Props,
    _answer: Option<&View>,
    width: usize,
    own: impl Fn(&Props, usize) -> Vec<Line<'static>>,
) -> Vec<Line<'static>> {
    own(props, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::component::InfoNoticeProps;
    use serde_json::json;
    fn own(props: &Props, _: usize) -> Vec<Line<'static>> {
        vec![Line::from(props.value()["text"].as_str().unwrap_or("own").to_owned())]
    }
    #[test]
    fn engine_uses_rewritten_props_inside_a_box() {
        let props = Props::InfoNotice(InfoNoticeProps { text:"original".into(), command:None });
        assert_eq!(draw(&props, None, 80, own)[0].to_string(), "original");
        let answer: View = serde_json::from_value(json!({"kind":"Box","children":[
            {"kind":"Text","children":["before"]},
            {"kind":"Engine","component":"InfoNotice","props":{"text":"rewritten","command":null}}
        ]})).unwrap();
        assert_eq!(draw(&props, Some(&answer), 80, own).iter().map(ToString::to_string).collect::<Vec<_>>(), ["before", "rewritten"]);
    }
    #[test]
    fn invalid_engine_props_draw_the_original_and_one_notice() {
        let props = Props::InfoNotice(InfoNoticeProps {text:"original".into(),command:None});
        let answer = View::Engine {component: props.component(),props:json!({"text":42})};
        let lines = draw(&props, Some(&answer), 80, own);
        assert_eq!(lines[0].to_string(), "original");
        assert_eq!(lines.len(), 2);
        assert!(lines[1].to_string().contains("invalid component props"));
    }
}
