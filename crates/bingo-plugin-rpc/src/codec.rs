//! Plugin protocol error vocabulary.
pub use bingo_jsonrpc::codec::*;
pub const INTERNAL_ERROR: i64 = -32603;
pub const TRANSPORT_ERROR: i64 = -32010;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn line(message: &Message) -> String {
        message.line().expect("a message serialises")
    }

    fn parse(raw: &str) -> Message {
        serde_json::from_str(raw).expect("a well formed message parses")
    }

    #[test]
    fn a_request_round_trips() {
        let request = Message::Request(Request::new(
            Id::Number(1),
            "tool/call",
            json!({ "callId": "call_1", "name": "count" }),
        ));
        assert_eq!(
            line(&request),
            r#"{"jsonrpc":"2.0","id":1,"method":"tool/call","params":{"callId":"call_1","name":"count"}}"#
        );
        assert_eq!(parse(&line(&request)), request);
    }

    #[test]
    fn a_result_and_an_error_are_never_both_present() {
        let ok = Message::Response(Response::ok(Id::Number(1), json!({ "output": {} })));
        assert_eq!(
            line(&ok),
            r#"{"jsonrpc":"2.0","id":1,"result":{"output":{}}}"#
        );
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
        let progress = Message::Notification(Notification::new(
            "tool/progress",
            json!({ "callId": "call_1", "tail": "counting" }),
        ));
        assert_eq!(
            line(&progress),
            r#"{"jsonrpc":"2.0","method":"tool/progress","params":{"callId":"call_1","tail":"counting"}}"#
        );
        assert_eq!(parse(&line(&progress)), progress);
    }

    #[test]
    fn another_version_is_not_a_message() {
        let wrong = r#"{"jsonrpc":"1.0","id":1,"method":"initialize","params":{}}"#;
        assert!(serde_json::from_str::<Message>(wrong).is_err());
    }

    #[test]
    fn absent_params_parse_as_null() {
        let Message::Request(request) = parse(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
        else {
            panic!("a request with an id and a method is a request");
        };
        assert_eq!(request.params, Value::Null);
    }
}
