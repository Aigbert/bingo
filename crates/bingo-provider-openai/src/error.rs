//! Failure classification. A status, a body and a `retry-after` header in,
//! one `ProviderError` out.
//!
//! The provider never retries: the turn loop owns the ladder and reads
//! `ProviderError::retryable` and `retry_after_ms`, so everything this module
//! decides is *what kind of failure this was*. Pure — no I/O, no client.

use bingo_sdk::ProviderError;
use bingo_sdk::provider::error::{message_of, retry_after_ms};
use serde_json::Value;

/// The code a 400 carries when the conversation outgrew the window. The loop
/// compacts and re-runs the turn instead of failing it.
const OVERFLOW_CODE: &str = "context_length_exceeded";

/// The same condition as a sentence, for an endpoint that names no code.
const OVERFLOW_PHRASES: &[&str] = &[OVERFLOW_CODE, "maximum context length"];

/// Error codes that mean the credentials, not the request.
const AUTH_CODES: &[&str] = &[
    "invalid_api_key",
    "invalid_authentication",
    "authentication_error",
    "account_deactivated",
];

/// Error codes that mean the server, and are worth another attempt.
const SERVER_CODES: &[&str] = &["server_error", "server_is_overloaded", "overloaded_error"];

/// One non-success HTTP response → the error the turn loop reacts to.
pub fn classify(status: u16, body: &str, retry_after: Option<&str>) -> ProviderError {
    let value = serde_json::from_str::<Value>(body).unwrap_or(Value::Null);
    let message = message_of(body);
    if status == 400 && says_overflow(code_of(&value), &message) {
        return ProviderError::ContextOverflow { message };
    }
    match status {
        401 | 403 => ProviderError::Auth { message },
        408 => ProviderError::Timeout,
        429 => ProviderError::RateLimited {
            retry_after_ms: retry_after
                .and_then(|value| retry_after_ms(value, jiff::Timestamp::now()))
                .or_else(|| body_delay_ms(&value)),
        },
        500..600 => ProviderError::Server { status, message },
        _ => ProviderError::Request { message },
    }
}

/// A `response.failed` or `error` event → the error the stream ends with.
/// There is no `ModelEvent::Error`: a failure the server announces mid-body
/// leaves the stream exactly as a failed request does.
pub fn stream_error(
    code: Option<&str>,
    message: &str,
    retry_after_ms: Option<u64>,
) -> ProviderError {
    if says_overflow(code, message) {
        // Verbatim: the kernel reads the real window out of this sentence.
        return ProviderError::ContextOverflow {
            message: message.to_string(),
        };
    }
    let message = match code {
        Some(code) => format!("{code}: {message}"),
        None => message.to_string(),
    };
    match code {
        Some(code) if AUTH_CODES.contains(&code) => ProviderError::Auth { message },
        Some(code) if SERVER_CODES.contains(&code) => ProviderError::Server {
            status: 500,
            message,
        },
        Some(code) if code.contains("rate_limit") => ProviderError::RateLimited { retry_after_ms },
        _ => ProviderError::Request { message },
    }
}

/// Whether the failure names the context window as the thing that was
/// exceeded, by code or by sentence.
fn says_overflow(code: Option<&str>, message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    code == Some(OVERFLOW_CODE) || OVERFLOW_PHRASES.iter().any(|p| message.contains(p))
}

/// `code` names the condition; `type` is the coarse family an error object
/// without a code still carries.
pub fn code_in(error: &Value) -> Option<&str> {
    error
        .get("code")
        .or_else(|| error.get("type"))
        .and_then(Value::as_str)
}

/// The same, out of a `{"error": {…}}` envelope or a body that is already
/// the error itself.
pub fn code_of(body: &Value) -> Option<&str> {
    code_in(body.get("error").unwrap_or(body))
}

/// The delay some endpoints put in the body instead of the header:
/// `retry_after_ms` in milliseconds, `retry_after` in seconds, at the top
/// level or inside the error envelope.
pub fn body_delay_ms(value: &Value) -> Option<u64> {
    field(value, "retry_after_ms")
        .and_then(|delay| millis(delay, 1.0))
        .or_else(|| field(value, "retry_after").and_then(|delay| millis(delay, 1_000.0)))
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .get(key)
        .or_else(|| value.get("error").and_then(|error| error.get(key)))
}

/// A JSON number or a numeric string, in units of `unit_ms` milliseconds.
fn millis(value: &Value, unit_ms: f64) -> Option<u64> {
    let raw = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))?;
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }
    Some((raw * unit_ms).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn body(code: &str, message: &str) -> String {
        json!({ "error": { "message": message, "type": "invalid_request_error", "code": code } })
            .to_string()
    }

    #[test]
    fn credentials_are_an_auth_failure_and_never_retried() {
        for status in [401, 403] {
            let error = classify(status, &body("invalid_api_key", "Incorrect API key"), None);
            assert_eq!(
                error,
                ProviderError::Auth {
                    message: "Incorrect API key".into()
                }
            );
            assert!(!error.retryable());
        }
    }

    #[test]
    fn a_429_reads_the_delay_from_the_header_or_the_body() {
        assert_eq!(
            classify(429, "{}", Some("30")),
            ProviderError::RateLimited {
                retry_after_ms: Some(30_000)
            }
        );
        assert_eq!(
            classify(429, r#"{"error":{"retry_after_ms":250}}"#, None),
            ProviderError::RateLimited {
                retry_after_ms: Some(250)
            }
        );
        assert_eq!(
            classify(429, r#"{"retry_after":1.5}"#, None),
            ProviderError::RateLimited {
                retry_after_ms: Some(1_500)
            }
        );
        assert_eq!(
            classify(429, r#"{"retry_after_ms":"400"}"#, None),
            ProviderError::RateLimited {
                retry_after_ms: Some(400)
            }
        );
    }

    #[test]
    fn the_header_wins_over_the_body_and_neither_is_required() {
        assert_eq!(
            classify(429, r#"{"retry_after_ms":250}"#, Some("7")),
            ProviderError::RateLimited {
                retry_after_ms: Some(7_000)
            }
        );
        assert_eq!(
            classify(429, "{}", None),
            ProviderError::RateLimited {
                retry_after_ms: None
            }
        );
    }

    #[test]
    fn a_400_naming_the_context_length_is_an_overflow_kept_verbatim() {
        let sentence = "This model's maximum context length is 400000 tokens. However, your messages resulted in 431201 tokens.";
        for payload in [
            body("context_length_exceeded", sentence),
            body("invalid_request_error", sentence),
        ] {
            let error = classify(400, &payload, None);
            assert_eq!(
                error,
                ProviderError::ContextOverflow {
                    message: sentence.into()
                },
                "the kernel parses the window out of this message"
            );
            assert!(!error.retryable(), "overflow is compacted, not retried");
        }
    }

    #[test]
    fn other_client_errors_are_bad_requests() {
        for status in [400, 404, 422] {
            assert_eq!(
                classify(
                    status,
                    &body("invalid_value", "Unknown parameter: 'foo'"),
                    None
                ),
                ProviderError::Request {
                    message: "Unknown parameter: 'foo'".into()
                },
                "status {status}"
            );
        }
    }

    #[test]
    fn server_errors_are_retryable_and_a_408_is_a_timeout() {
        for status in [500, 502, 503] {
            let error = classify(
                status,
                &body("server_error", "The server had an error"),
                None,
            );
            assert_eq!(
                error,
                ProviderError::Server {
                    status,
                    message: "The server had an error".into()
                }
            );
            assert!(error.retryable());
        }
        assert_eq!(classify(408, "", None), ProviderError::Timeout);
    }

    #[test]
    fn a_stream_error_is_named_by_its_code() {
        assert_eq!(
            stream_error(Some("server_is_overloaded"), "try later", None),
            ProviderError::Server {
                status: 500,
                message: "server_is_overloaded: try later".into()
            }
        );
        assert_eq!(
            stream_error(Some("rate_limit_exceeded"), "slow down", Some(900)),
            ProviderError::RateLimited {
                retry_after_ms: Some(900)
            }
        );
        assert!(matches!(
            stream_error(Some("invalid_api_key"), "expired", None),
            ProviderError::Auth { .. }
        ));
        assert!(matches!(
            stream_error(None, "something went wrong", None),
            ProviderError::Request { .. }
        ));
    }

    #[test]
    fn a_stream_overflow_keeps_the_sentence_the_kernel_measures() {
        let sentence = "This model's maximum context length is 128000 tokens.";
        assert_eq!(
            stream_error(Some("context_length_exceeded"), sentence, None),
            ProviderError::ContextOverflow {
                message: sentence.into()
            },
            "no code prefix: the window is read out of this string"
        );
    }

    #[test]
    fn the_code_falls_back_to_the_error_type() {
        assert_eq!(
            code_of(&json!({ "error": { "type": "invalid_request_error" } })),
            Some("invalid_request_error")
        );
        assert_eq!(
            code_of(&json!({ "error": { "type": "x", "code": "rate_limit_exceeded" } })),
            Some("rate_limit_exceeded"),
            "the code is more precise than the family"
        );
        assert_eq!(
            code_of(&json!({ "code": "server_error" })),
            Some("server_error"),
            "an `error` event is already the error itself"
        );
        assert_eq!(code_of(&json!({ "error": {} })), None);
    }

    #[test]
    fn a_nonsense_delay_is_no_delay() {
        assert_eq!(body_delay_ms(&json!({ "retry_after_ms": -1 })), None);
        assert_eq!(body_delay_ms(&json!({ "retry_after": "soon" })), None);
        assert_eq!(body_delay_ms(&json!({})), None);
    }
}
