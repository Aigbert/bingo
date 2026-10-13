//! The envelope: one JSON-RPC 2.0 message per line (ADR-0007).
//!
//! Nothing here knows what a method does; it knows what a message looks like.

use std::fmt;

pub use crate::lines::{CodecError, Lines, lines};
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// The only version this surface speaks.
const VERSION: &str = "2.0";

/// A frame can carry a base64 image, so the line limit is generous.
pub const MAX_LINE: usize = 16 * 1024 * 1024;

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
/// `"2.0"`. A message with any other version cannot be built or parsed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Version;

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(VERSION)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        if raw == VERSION {
            Ok(Version)
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(&raw), &VERSION))
        }
    }
}

/// A request id: a number or a string, echoed verbatim in the response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    Number(i64),
    String(String),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Number(n) => write!(f, "{n}"),
            Id::String(s) => f.write_str(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: Version,
    pub id: Id,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    pub fn new(id: Id, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: Version,
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: Version,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Notification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: Version,
            method: method.into(),
            params,
        }
    }
}

/// A reply carries a result or an error, never both and never neither.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Result(Value),
    Error(RpcError),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: Version,
    /// `null` when the line could not be parsed far enough to carry one.
    pub id: Option<Id>,
    #[serde(flatten)]
    pub outcome: Outcome,
}

impl Response {
    pub fn ok(id: Id, result: Value) -> Self {
        Self {
            jsonrpc: Version,
            id: Some(id),
            outcome: Outcome::Result(result),
        }
    }

    pub fn failed(id: Option<Id>, error: RpcError) -> Self {
        Self {
            jsonrpc: Version,
            id,
            outcome: Outcome::Error(error),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

/// Any of the three, from one line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl Message {
    pub fn line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
