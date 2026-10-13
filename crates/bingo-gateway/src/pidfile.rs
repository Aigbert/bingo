//! The gateway's readable process record. Exclusion belongs to `bingo-claim`.
//! Legacy records have no `claimed` field and still require a PID probe.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// What the resident process wrote down about itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub pid: u32,
    #[serde(default)]
    pub claimed: bool,
    /// The binary that is running, so `status` can say when it is older than
    /// the one that would be started now.
    pub version: String,
    pub started: Timestamp,
}

impl Record {
    /// This process, as of `started`.
    pub fn here(started: Timestamp) -> Self {
        Self {
            pid: std::process::id(),
            claimed: true,
            version: version().to_string(),
            started,
        }
    }
}

/// The binary running this code, which `status` and `doctor` compare against
/// the record a resident process left.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A record as it is written: JSON, because a person reads this file when
/// something has gone wrong and a bare number would answer none of it.
pub fn render(record: &Record) -> String {
    // A record contains only owned scalars; the only way this fails is a bug, and
    // an empty pidfile would be worse than the fallback line.
    match serde_json::to_string_pretty(record) {
        Ok(json) => format!("{json}\n"),
        Err(e) => format!("{{\"pid\":{},\"error\":\"{e}\"}}\n", record.pid),
    }
}

pub fn parse(text: &str) -> Result<Record, String> {
    serde_json::from_str(text).map_err(|e| format!("{e}"))
}

/// The record in `path`; `None` when no gateway ever wrote one there.
pub fn read(path: &Path) -> Result<Option<Record>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    parse(&text)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The permanent claim protocol; the process record is stored separately.
pub const PROTOCOL: bingo_claim::Protocol<'static> = bingo_claim::Protocol {
    file: "gateway.lock",
    marker: "bingo-gateway-v1\n",
};

/// Remove the record before releasing the resident's OS claim.
#[derive(Debug)]
pub struct Published(PathBuf);

impl Published {
    pub fn write(path: &Path, record: &Record) -> Result<Self, String> {
        std::fs::write(path, render(record)).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self(path.to_path_buf()))
    }
}

impl Drop for Published {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> Record {
        Record {
            pid: 4242,
            claimed: false,
            version: "0.1.0".into(),
            started: "2026-08-31T09:00:00Z".parse().expect("a timestamp"),
        }
    }

    #[test]
    fn a_record_survives_the_round_trip_through_the_file_it_is_written_as() {
        let rendered = render(&record());
        assert_eq!(parse(&rendered).expect("it parses"), record());
        assert!(rendered.ends_with('\n'), "{rendered}");
        assert!(rendered.contains("\"pid\": 4242"), "{rendered}");
        assert!(
            rendered.contains("2026-08-31T09:00:00Z"),
            "the start time is readable, not a count of seconds: {rendered}"
        );
    }

    #[test]
    fn a_file_that_is_not_a_record_is_an_error_naming_the_path() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("gateway.pid");
        assert_eq!(read(&path).expect("no file is no record"), None);
        std::fs::write(&path, "4242").expect("a bare pid, as an older tree wrote");
        let refused = read(&path).expect_err("a refusal");
        assert!(refused.contains("gateway.pid"), "{refused}");
    }

    #[test]
    fn the_record_this_process_writes_names_this_process_and_this_binary() {
        let now = Timestamp::now();
        let here = Record::here(now);
        assert_eq!(here.pid, std::process::id());
        assert_eq!(here.version, version());
        assert_eq!(here.started, now);
    }
}
