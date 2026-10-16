//! Journal projections shared by the CLI fixtures.
use bingo_sdk::{ContentPart, Event, Frame, ItemBody, Origin, ToolOutput};
use std::path::{Path, PathBuf};

pub(super) fn sessions(home: &Path) -> PathBuf {
    home.join(".bingo/data/sessions")
}

pub(super) fn dirs(home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(sessions(home)) else {
        return Vec::new();
    };
    entries.flatten().map(|entry| entry.path()).collect()
}

pub(super) fn summary_of(dir: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(dir.join("summary.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn keyed(dir: &Path, key: &str) -> bool {
    summary_of(dir).is_some_and(|summary| summary["key"] == key)
}

pub(super) fn session_dir(home: &Path, key: &str) -> Option<PathBuf> {
    dirs(home).into_iter().find(|dir| keyed(dir, key))
}

pub(super) fn frames_at(dir: &Path) -> Vec<Frame> {
    let Ok(text) = std::fs::read_to_string(dir.join("journal.jsonl")) else {
        return Vec::new();
    };
    text.lines()
        .skip(1)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

pub(super) fn posts(frames: &[Frame]) -> Vec<(String, Origin)> {
    frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::ItemCompleted { item } => match &item.body {
                ItemBody::User { parts, origin } => Some((
                    parts.iter().filter_map(ContentPart::as_text).collect(),
                    origin.clone(),
                )),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

pub(super) fn text_of(output: &ToolOutput) -> String {
    output
        .parts
        .iter()
        .filter_map(ContentPart::as_text)
        .collect()
}

pub(super) fn journal(home: &Path, key: &str) -> Vec<Frame> {
    let dir = session_dir(home, key).unwrap_or_else(|| panic!("no session is keyed {key}"));
    let text = std::fs::read_to_string(dir.join("journal.jsonl")).expect("a journal on disk");
    text.lines()
        .skip(1)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

pub(super) fn results(frames: &[Frame], tool: &str) -> Vec<ToolOutput> {
    frames
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::ItemCompleted { item } => match &item.body {
                ItemBody::ToolCall { name, output, .. } if name == tool => output.clone(),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

pub(super) fn root_dir(home: &Path) -> Option<PathBuf> {
    dirs(home)
        .into_iter()
        .find(|dir| summary_of(dir).is_some_and(|summary| summary["parent"].is_null()))
}

pub(super) fn room_dir(home: &Path) -> Option<PathBuf> {
    dirs(home).into_iter().find(|dir| {
        summary_of(dir).is_some_and(|summary| {
            summary["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("rooms/"))
        })
    })
}
