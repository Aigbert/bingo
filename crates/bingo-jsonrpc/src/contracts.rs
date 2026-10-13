use crate::codec::*;
use serde_json::{Value, json};
    fn line(message: &Message) -> String {
        serde_json::to_string(message).expect("a message serialises")
    }

    fn parse(raw: &str) -> Message {
        serde_json::from_str(raw).expect("a well formed message parses")
    }

    #[test]
    fn a_request_round_trips() {
        let request = Message::Request(Request::new(
            Id::Number(7),
            "session/open",
            json!({ "selector": {} }),
        ));
        assert_eq!(
            line(&request),
            r#"{"jsonrpc":"2.0","id":7,"method":"session/open","params":{"selector":{}}}"#
        );
        assert_eq!(parse(&line(&request)), request);
    }

    #[test]
    fn a_string_id_stays_a_string() {
        let request = Message::Request(Request::new(Id::String("a".into()), "shutdown", json!({})));
        assert_eq!(parse(&line(&request)), request);
    }

    #[test]
    fn a_result_and_an_error_are_never_both_present() {
        let ok = Message::Response(Response::ok(Id::Number(1), json!({})));
        assert_eq!(line(&ok), r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        let failed = Message::Response(Response::failed(
            None,
            RpcError::new(PARSE_ERROR, "not json"),
        ));
        assert_eq!(
            line(&failed),
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"not json"}}"#
        );
        assert_eq!(parse(&line(&failed)), failed);
    }

    #[test]
    fn a_notification_has_no_id() {
        let event = Message::Notification(Notification::new("event", json!({ "seq": 1 })));
        assert_eq!(
            line(&event),
            r#"{"jsonrpc":"2.0","method":"event","params":{"seq":1}}"#
        );
        assert_eq!(parse(&line(&event)), event);
    }

    #[test]
    fn another_version_is_not_a_message() {
        let wrong = r#"{"jsonrpc":"1.0","id":1,"method":"shutdown","params":{}}"#;
        assert!(serde_json::from_str::<Message>(wrong).is_err());
    }

    #[test]
    fn absent_params_parse_as_null() {
        let Message::Request(request) = parse(r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#)
        else {
            panic!("a request with an id and a method is a request");
        };
        assert_eq!(request.params, Value::Null);
    }

