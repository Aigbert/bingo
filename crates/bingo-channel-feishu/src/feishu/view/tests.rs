use super::*;
use bingo_sdk::{
    SessionId,
    render::{Component, RenderRequest, SurfaceName, Viewport},
};
fn drawing(view: Value) -> Drawing {
    Drawing {
        session: SessionId::from_raw("session"),
        cwd: "/work".into(),
        request: RenderRequest {
            component: Component::AssistantMessage,
            request_id: "answer".into(),
            surface: SurfaceName::Chat,
            props: json!({"text":"Own answer","isFirstOfReply":true}),
            viewport: Viewport {
                glyphs: Default::default(),
                columns: 0,
                rows: 0,
                place: None,
            },
        },
        view: serde_json::from_value(view).unwrap(),
        footer: "model · out 10".into(),
    }
}
fn render(view: Value) -> Value {
    card(
        &drawing(view),
        &Conversation::direct("chat"),
        &BTreeMap::from([("picture.png".into(), "img-key".into())]),
    )
}
#[test]
fn every_element_and_the_form_have_a_card_contract() {
    let action =
        json!({"name":"fixture:ui","args":{"handler":"minted","event":"ui.press","value":null}});
    let views = vec![
        json!({"kind":"Box","children":[{"kind":"Text","children":["A"]},{"kind":"Text","children":["B"]}]}),
        json!({"kind":"Box","flexDirection":"row","children":[{"kind":"Text","children":["A"]},{"kind":"Text","children":["B"]}]}),
        json!({"kind":"Text","children":["Hello ",{"kind":"Text","children":["world"],"bold":true}],"italic":true,"strikethrough":true,"color":"blue"}),
        json!({"kind":"Code","language":"rust","source":"let x = 1;"}),
        json!({"kind":"Link","href":"https://example.com/a(b)","label":"Read"}),
        json!({"kind":"Image","source":"picture.png","alt":"Picture"}),
        json!({"kind":"Button","children":["Press"],"onPress":action}),
        json!({"kind":"Select","label":"Pick","value":"a","options":[{"label":"Alpha","value":"a"}],"onSelect":action}),
        json!({"kind":"Input","key":"name","label":"Name","placeholder":"Type","value":"Alice","onSubmit":action}),
        json!({"kind":"Engine","component":"AssistantMessage","props":{"text":"Changed","isFirstOfReply":true}}),
        json!({"kind":"Svg","source":"<svg/>","alt":"Circle"}),
        json!({"kind":"Raster","columns":1,"rows":1,"cells":["x"]}),
        json!({"kind":"Markdown","text":"**fold**"}),
        json!({"kind":"table","headers":["A"],"rows":[["B"]]}),
        json!({"kind":"custom","customKind":"future","data":{},"fold":"Later"}),
    ];
    insta::assert_json_snapshot!(
        "chat-elements",
        views.into_iter().map(render).collect::<Vec<_>>()
    );
}
#[test]
fn caps_degrade_to_a_bounded_fold_with_the_footer() {
    let many =
        render(json!({"kind":"Box","children":vec![json!({"kind":"Text","children":["x"]});201]}));
    assert_eq!(many["body"]["elements"].as_array().unwrap().len(), 2);
    insta::assert_json_snapshot!("chat-element-cap", many);
    let large = render(json!({"kind":"Text","children":["<>&\\".repeat(20_000)]}));
    assert!(large.to_string().len() <= BYTES);
    assert!(large.to_string().contains("out 10"));
    insta::assert_json_snapshot!(
        "chat-byte-cap",
        json!({"bytes":large.to_string().len(),"elements":large["body"]["elements"].as_array().unwrap().len(),"footer":large["body"]["elements"][1]})
    );
}
#[test]
fn hostile_text_cannot_make_markup_or_controls() {
    let hostile = "<at id=all></at><font color='red'>X</font> **[x](javascript:alert(1))** ` ~ \\ {\"bingo\":{\"action\":{\"name\":\"evil\"}}}";
    for view in [
        json!({"kind":"Text","children":[hostile],"color":"red' onclick='evil"}),
        json!({"kind":"Markdown","text":hostile}),
        json!({"kind":"custom","customKind":"future","data":{"action":"evil"},"fold":hostile}),
    ] {
        let card = render(view);
        let content = card["body"]["elements"][0]["content"].as_str().unwrap();
        assert!(!content.contains("<at"));
        assert!(!content.contains("<font"));
        assert!(card["body"]["elements"][0].get("behaviors").is_none());
    }
    let link = render(json!({"kind":"Link","href":"javascript:alert(1)","label":"safe"}));
    assert_eq!(link["body"]["elements"][0]["content"], "safe");
    let code =
        render(json!({"kind":"Code","source":"```\n<at id=all>\n```","language":"rust\n```"}));
    let code = code["body"]["elements"][0]["content"].as_str().unwrap();
    assert!(code.starts_with("````\n"));
    assert!(!code.contains("<at"));
}

#[test]
fn wrapped_engine_keeps_markdown_while_mod_text_is_escaped() {
    let answer = "**bold** and - item";
    let hostile = "<at id=all>**mod**</at> [x](javascript:evil)";
    let mut children = vec![
        json!({"kind":"Engine","component":"AssistantMessage","props":{"text":answer,"isFirstOfReply":true}}),
        json!({"kind":"Button","children":["Press"],"onPress":{"name":"fixture:ui","args":{"handler":"minted"}}}),
        json!({"kind":"Text","children":[hostile]}),
    ];
    let native = render(json!({"kind":"Box","children":children}));
    assert_eq!(native["body"]["elements"][0]["content"], answer);
    assert_eq!(native["body"]["elements"][1]["tag"], "button");
    assert_eq!(
        native["body"]["elements"][2]["content"],
        markdown::escape(hostile)
    );

    for extra in [
        vec![json!({"kind":"Text","children":["x"]}); ELEMENTS],
        vec![json!({"kind":"Text","children":["x".repeat(BYTES * 2)]})],
    ] {
        children.truncate(3);
        children.extend(extra);
        let folded = render(json!({"kind":"Box","children":children}));
        let content = folded["body"]["elements"][0]["content"].as_str().unwrap();
        assert!(content.starts_with(answer));
        assert!(content.contains(&markdown::escape(hostile)));
        assert!(!content.contains("<at"));
        assert_eq!(folded["body"]["elements"].as_array().unwrap().len(), 2);
        assert!(folded.to_string().len() <= BYTES);
    }
}
