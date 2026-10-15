//! Live facts and forwarding configuration, supplied by the MCP owner.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    Waiting { changed: bool },
    Connecting,
    Connected { tools: usize },
    SigningIn,
    NeedsAuth { why: String },
    Failed { why: String },
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Auth {
    SignedOut,
    SignedIn,
    Expired { reason: String },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}
impl Transport {
    pub fn name(&self) -> &str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
        }
    }
    /// What a person reads it runs: tokens a line carries are cut out.
    pub fn runs(&self) -> String {
        match self {
            Self::Stdio { command, args, .. } => std::iter::once(command.clone())
                .chain(redacted_args(args))
                .collect::<Vec<_>>()
                .join(" "),
            Self::Http { url, .. } => redacted_url(url),
        }
    }
    /// What a row may carry when nobody may dial it: the program or the
    /// host, never its arguments, environment, headers, path or query.
    pub fn redacted(&self) -> Self {
        match self {
            Self::Stdio { command, .. } => Self::Stdio {
                command: command.clone(),
                args: vec![],
                env: BTreeMap::new(),
                cwd: None,
            },
            Self::Http { url, .. } => Self::Http {
                url: origin(url),
                headers: BTreeMap::new(),
            },
        }
    }
    /// What a row carries while it waits for the person's yes: everything
    /// the repository wrote that a yes would run, with the values of its
    /// environment and headers hidden and their names kept.
    pub fn reviewed(&self) -> Self {
        let hidden = |values: &BTreeMap<String, String>| {
            values
                .keys()
                .map(|name| (name.clone(), HIDDEN.to_string()))
                .collect()
        };
        match self {
            Self::Stdio {
                command,
                args,
                env,
                cwd,
            } => Self::Stdio {
                command: command.clone(),
                args: args.clone(),
                env: hidden(env),
                cwd: cwd.clone(),
            },
            Self::Http { url, headers } => Self::Http {
                url: url.clone(),
                headers: hidden(headers),
            },
        }
    }
    /// The whole line as written, for the person deciding whether to run it.
    pub fn line(&self) -> String {
        match self {
            Self::Stdio { command, args, .. } => std::iter::once(command.as_str())
                .chain(args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            Self::Http { url, .. } => url.clone(),
        }
    }
    /// The names of the environment or headers it is given, never values.
    pub fn named(&self) -> Vec<&str> {
        match self {
            Self::Stdio { env, .. } => env.keys().map(String::as_str).collect(),
            Self::Http { headers, .. } => headers.keys().map(String::as_str).collect(),
        }
    }
}
const HIDDEN: &str = "…";
/// A line of free text a server wrote, with what looks like a token cut out:
/// the word after a credential's name too, since prose has no dashes.
pub fn redacted_line(line: &str) -> String {
    let mut shown = Vec::new();
    let mut hide_next = false;
    for word in line.split_whitespace() {
        shown.push(if std::mem::take(&mut hide_next) {
            HIDDEN.to_string()
        } else {
            redacted_arg(word)
        });
        hide_next = !word.contains('=') && sensitive(word);
    }
    shown.join(" ")
}
fn sensitive(name: &str) -> bool {
    let name = name.trim_start_matches('-').to_ascii_lowercase();
    [
        "token",
        "key",
        "secret",
        "password",
        "auth",
        "bearer",
        "credential",
    ]
    .iter()
    .any(|word| name.contains(word))
}
fn env_like(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
fn redacted_args(args: &[String]) -> Vec<String> {
    let mut shown = Vec::with_capacity(args.len());
    let mut hide_next = false;
    for arg in args {
        shown.push(if std::mem::take(&mut hide_next) {
            HIDDEN.to_string()
        } else {
            redacted_arg(arg)
        });
        hide_next = arg.starts_with('-') && !arg.contains('=') && sensitive(arg);
    }
    shown
}
fn redacted_arg(arg: &str) -> String {
    if arg.contains("://") {
        return redacted_url(arg);
    }
    match arg.split_once('=') {
        Some((name, _)) if env_like(name) || sensitive(name) => format!("{name}={HIDDEN}"),
        _ => arg.to_string(),
    }
}
fn split_url(url: &str) -> (&str, &str, &str) {
    let Some((scheme, rest)) = url.split_once("://") else {
        return ("", "", url);
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (scheme, host, tail)
}
fn origin(url: &str) -> String {
    match split_url(url) {
        ("", _, _) => String::new(),
        (scheme, host, _) => format!("{scheme}://{host}"),
    }
}
fn redacted_url(url: &str) -> String {
    let (scheme, host, tail) = split_url(url);
    let tail = tail.split_once('#').map_or(tail, |(before, _)| before);
    let tail = match tail.split_once('?') {
        Some((path, query)) => format!(
            "{path}?{}",
            query
                .split('&')
                .map(|pair| match pair.split_once('=') {
                    Some((name, _)) => format!("{name}={HIDDEN}"),
                    None => pair.to_string(),
                })
                .collect::<Vec<_>>()
                .join("&")
        ),
        None => tail.to_string(),
    };
    if scheme.is_empty() {
        return tail;
    }
    format!("{scheme}://{host}{tail}")
}
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("type", &self.name())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerRow {
    pub name: String,
    pub transport: Transport,
    pub origin: Option<String>,
    pub status: Status,
    pub auth: Option<Auth>,
    pub latency: Option<u64>,
    pub sign_in_from: Option<String>,
    /// The last line a failed server wrote to its stderr log, redacted.
    #[serde(default)]
    pub stderr: Option<String>,
}
impl ServerRow {
    pub fn tools(&self) -> usize {
        match self.status {
            Status::Connected { tools } => tools,
            _ => 0,
        }
    }
    pub fn forwardable(&self) -> bool {
        !matches!(self.status, Status::Disabled | Status::Waiting { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stdio(args: &[&str]) -> Transport {
        Transport::Stdio {
            command: "npx".into(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: BTreeMap::from([("TOKEN".into(), "s3cret".into())]),
            cwd: Some("/home/me/secret-project".into()),
        }
    }
    fn http(url: &str) -> Transport {
        Transport::Http {
            url: url.into(),
            headers: BTreeMap::from([("Authorization".into(), "Bearer s3cret".into())]),
        }
    }
    #[test]
    fn what_a_row_runs_never_shows_a_token() {
        assert_eq!(
            stdio(&[
                "-y",
                "files",
                "API_KEY=s3cret",
                "--token",
                "s3cret",
                "--auth=s3cret"
            ])
            .runs(),
            "npx -y files API_KEY=… --token … --auth=…"
        );
        assert_eq!(
            http("https://me:s3cret@mcp.example.com/mcp?key=s3cret&debug#s3cret").runs(),
            "https://mcp.example.com/mcp?key=…&debug"
        );
        assert_eq!(
            stdio(&["--url", "https://u:s3cret@h.example/x?t=s3cret"]).runs(),
            "npx --url https://h.example/x?t=…"
        );
    }
    #[test]
    fn a_line_a_server_wrote_never_shows_a_token() {
        assert_eq!(
            redacted_line(
                "error: Authorization: Bearer s3cret refused by https://u:s3cret@h.example/x?k=s3cret API_KEY=s3cret"
            ),
            "error: Authorization: … … refused by https://h.example/x?k=… API_KEY=…"
        );
        assert_eq!(
            redacted_line("  cannot find module 'x'  "),
            "cannot find module 'x'"
        );
    }
    #[test]
    fn a_redacted_row_keeps_only_the_program_or_the_host() {
        assert_eq!(
            stdio(&["--token", "s3cret"]).redacted(),
            stdio(&[]).redacted()
        );
        let Transport::Stdio { args, env, cwd, .. } = stdio(&["x"]).redacted() else {
            panic!("stdio stays stdio")
        };
        assert!(args.is_empty() && env.is_empty() && cwd.is_none());
        assert_eq!(
            http("https://me:s3cret@mcp.example.com:8443/mcp/s3cret?key=s3cret").redacted(),
            Transport::Http {
                url: "https://mcp.example.com:8443".into(),
                headers: BTreeMap::new()
            }
        );
    }
    #[test]
    fn a_reviewed_row_keeps_the_line_and_only_the_names_of_its_secrets() {
        let Transport::Stdio {
            command,
            args,
            env,
            cwd,
        } = stdio(&["--token", "s3cret", "files"]).reviewed()
        else {
            panic!("stdio stays stdio")
        };
        assert_eq!(command, "npx");
        assert_eq!(args, ["--token", "s3cret", "files"]);
        assert_eq!(env, BTreeMap::from([("TOKEN".into(), HIDDEN.into())]));
        assert_eq!(cwd, Some("/home/me/secret-project".into()));
        let web = http("https://mcp.example.com/mcp?debug").reviewed();
        assert_eq!(web.line(), "https://mcp.example.com/mcp?debug");
        assert_eq!(web.named(), ["Authorization"]);
        assert!(!format!("{:?}", serde_json::to_value(&web).unwrap()).contains("s3cret"));
    }
    /// The recorded rows nobody may dial are pinned in their redacted shape:
    /// an owner that answers more for them breaks the contract.
    #[test]
    fn rows_nobody_may_dial_are_recorded_redacted() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/mcp.json")).unwrap();
        let rows: Vec<ServerRow> =
            serde_json::from_value(fixture["answer"]["servers"].clone()).unwrap();
        let closed = rows
            .iter()
            .filter(|row| !row.forwardable())
            .collect::<Vec<_>>();
        assert_eq!(closed.len(), 2);
        for row in closed {
            let shape = match row.status {
                Status::Waiting { .. } => row.transport.reviewed(),
                _ => row.transport.redacted(),
            };
            assert_eq!(row.transport, shape, "{}", row.name);
        }
    }
}
