use super::*;
use crate::feishu::tests::{ok, signed_in};
use wiremock::matchers::{body_json, header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn entry(name: &str, description: &str) -> MenuEntry {
    MenuEntry {
        name: name.into(),
        description: description.into(),
    }
}
async fn setup() -> (MockServer, Api, tempfile::TempDir, Publisher) {
    let server = MockServer::start().await;
    signed_in(&server).await;
    let api = Api::new(
        bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
        server.uri(),
        "cli_a",
        "secret",
    );
    let dir = tempfile::tempdir().unwrap();
    let publisher = Publisher::new(&dir.path().join("attachments"), "cli_a");
    (server, api, dir, publisher)
}

#[tokio::test]
async fn create_patch_and_delete_only_owned_commands() {
    let (server, api, _dir, publisher) = setup().await;
    publisher
        .save(&BTreeMap::from([
            ("old".into(), "id-old".into()),
            ("model".into(), "id-model".into()),
        ]))
        .unwrap();
    ok(
        &server,
        "GET",
        PATH,
        json!({"items":[
            {"command_id":"id-old","command":"old","description":{"default_value":"old"}},
            {"command_id":"id-model","command":"model","description":{"default_value":"before"}},
            {"command_id":"id-person","command":"personal","description":{"default_value":"mine"}}
        ]}),
    )
    .await;
    for (verb, suffix, body, data) in [
        ("DELETE", "/id-old", None, json!({})),
        (
            "PATCH",
            "/id-model",
            Some(json!({"description":{"default_value":"after","i18n":{}}})),
            json!({}),
        ),
        (
            "POST",
            "",
            Some(json!({"command":"new","description":{"default_value":"fresh","i18n":{}}})),
            json!({"command_id":"id-new"}),
        ),
    ] {
        let mut mock = Mock::given(method(verb))
            .and(path(format!("{PATH}{suffix}")))
            .and(header("authorization", "Bearer t-1"));
        if let Some(body) = body {
            mock = mock.and(body_json(body));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":data})))
            .expect(1)
            .mount(&server)
            .await;
    }
    publisher
        .publish(
            &api,
            &[
                entry("model", "after"),
                entry("new", "fresh"),
                entry("personal", "changed"),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        publisher.read().unwrap(),
        BTreeMap::from([
            ("model".into(), "id-model".into()),
            ("new".into(), "id-new".into())
        ])
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 5);
}

#[tokio::test]
async fn a_missing_scope_stops_until_restart() {
    let (server, api, _dir, publisher) = setup().await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"code":99991672,"msg":"Access denied. Missing scope"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(
        publisher
            .publish(&api, &[entry("model", "Models")])
            .await
            .is_err()
    );
    publisher
        .publish(&api, &[entry("model", "Models")])
        .await
        .unwrap();
}

#[tokio::test]
async fn foreign_names_and_full_menus_are_never_overwritten() {
    let (server, api, _dir, publisher) = setup().await;
    let items: Vec<_> = (0..100).map(|i| json!({"command_id":format!("id-{i}"),"command":format!("c{i}"),"description":{"default_value":"person"}})).collect();
    ok(&server, "GET", PATH, json!({"items":items})).await;
    publisher
        .publish(&api, &[entry("c0", "changed"), entry("new", "new")])
        .await
        .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
    assert!(publisher.read().unwrap().is_empty());
}

#[tokio::test]
async fn at_most_one_hundred_commands_are_created_and_ownership_survives_restart() {
    let (server, api, dir, publisher) = setup().await;
    ok(&server, "GET", PATH, json!({"items":[]})).await;
    Mock::given(method("POST")).and(path(PATH)).respond_with(|request: &wiremock::Request| {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":{"command_id":format!("id-{}",body["command"].as_str().unwrap())}}))
    }).expect(100).mount(&server).await;
    let entries: Vec<_> = (0..105)
        .map(|i| entry(&format!("c{i}"), "Command"))
        .collect();
    publisher.publish(&api, &entries).await.unwrap();
    let restarted = Publisher::new(&dir.path().join("attachments"), "cli_a");
    assert_eq!(restarted.read().unwrap().len(), 100);
    assert!(!restarted.read().unwrap().contains_key("c100"));
}
#[tokio::test]
async fn a_duplicate_create_never_claims_a_foreign_command() {
    let (server, api, _dir, publisher) = setup().await;
    ok(&server, "GET", PATH, json!({"items":[]})).await;
    Mock::given(method("POST"))
        .and(path(PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"code":40000000,"msg":"command already exists"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    publisher
        .publish(&api, &[entry("model", "Models")])
        .await
        .unwrap();
    assert!(publisher.read().unwrap().is_empty());
}
#[tokio::test]
async fn an_owned_name_recreated_by_the_person_is_no_longer_owned() {
    let (server, api, _dir, publisher) = setup().await;
    publisher
        .save(&BTreeMap::from([("model".into(), "old-id".into())]))
        .unwrap();
    ok(&server,"GET",PATH,json!({"items":[{"command":"model","command_id":"new-id","description":{"default_value":"mine"}}]})).await;
    publisher.publish(&api, &[]).await.unwrap();
    assert!(publisher.read().unwrap().is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_refused_create_does_not_block_the_next_entry() {
    let (server, api, _dir, publisher) = setup().await;
    ok(&server, "GET", PATH, json!({"items":[]})).await;
    for (name, response) in [
        (
            "plugin:Command",
            json!({"code":40000001,"msg":"invalid command"}),
        ),
        ("valid", json!({"code":0,"data":{"command_id":"id-valid"}})),
    ] {
        Mock::given(method("POST"))
            .and(path(PATH))
            .and(body_json(
                json!({"command":name,"description":{"default_value":"Command","i18n":{}}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }
    publisher
        .publish(
            &api,
            &[
                entry("plugin:Command", "Command"),
                entry("valid", "Command"),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        publisher.read().unwrap(),
        BTreeMap::from([("valid".into(), "id-valid".into())])
    );
    assert!(
        publisher
            .state
            .lock()
            .await
            .warned
            .contains("plugin:Command")
    );
}

#[tokio::test]
async fn ownership_pruning_waits_for_every_remote_page() {
    let (server, api, _dir, publisher) = setup().await;
    publisher
        .save(&BTreeMap::from([("owned".into(), "id-owned".into())]))
        .unwrap();
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(query_param("page_size", "100"))
        .and(query_param_is_missing("page_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":{
                "items":[{"command":"personal","command_id":"id-person"}],
                "has_more":true,"page_token":"next+/=&page"
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(PATH))
        .and(query_param("page_size", "100"))
        .and(query_param("page_token", "next+/=&page"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":{
                "items":[{"command":"owned","command_id":"id-owned"}],"has_more":false
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{PATH}/id-owned")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":{}})))
        .expect(1)
        .mount(&server)
        .await;
    publisher.publish(&api, &[]).await.unwrap();
    assert!(publisher.read().unwrap().is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn a_refused_patch_does_not_block_the_next_entry() {
    let (server, api, _dir, publisher) = setup().await;
    publisher
        .save(&BTreeMap::from([("owned".into(), "id-owned".into())]))
        .unwrap();
    ok(&server, "GET", PATH, json!({"items":[{"command":"owned","command_id":"id-owned","description":{"default_value":"before"}}]})).await;
    Mock::given(method("PATCH"))
        .and(path(format!("{PATH}/id-owned")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"code":40000001,"msg":"invalid description"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(PATH))
        .and(body_json(
            json!({"command":"valid","description":{"default_value":"Command","i18n":{}}}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"code":0,"data":{"command_id":"id-valid"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    publisher
        .publish(&api, &[entry("owned", "after"), entry("valid", "Command")])
        .await
        .unwrap();
    assert_eq!(
        publisher.read().unwrap(),
        BTreeMap::from([
            ("owned".into(), "id-owned".into()),
            ("valid".into(), "id-valid".into())
        ])
    );
}

#[tokio::test]
async fn transport_errors_and_missing_scopes_still_stop_mutations() {
    for response in [
        ResponseTemplate::new(200).set_body_json(json!({"code":99991672,"msg":"Missing scope"})),
        ResponseTemplate::new(502).set_body_string("unavailable"),
    ] {
        let (server, api, _dir, publisher) = setup().await;
        ok(&server, "GET", PATH, json!({"items":[]})).await;
        Mock::given(method("POST"))
            .and(path(PATH))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        assert!(
            publisher
                .publish(
                    &api,
                    &[entry("first", "Command"), entry("second", "Command")]
                )
                .await
                .is_err()
        );
        assert!(publisher.read().unwrap().is_empty());
        assert!(publisher.state.lock().await.warned.is_empty());
    }
}

#[tokio::test]
async fn incomplete_pagination_never_prunes_ownership() {
    for token in [Value::Null, json!("repeated")] {
        let (server, api, _dir, publisher) = setup().await;
        let owned = BTreeMap::from([("owned".into(), "id-owned".into())]);
        publisher.save(&owned).unwrap();
        Mock::given(method("GET"))
            .and(path(PATH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"code":0,"data":{
                    "items":[],"has_more":true,"page_token":token
                }})),
            )
            .expect(if token.is_null() { 1 } else { 2 })
            .mount(&server)
            .await;
        assert!(publisher.publish(&api, &[]).await.is_err());
        assert_eq!(publisher.read().unwrap(), owned);
    }
}
