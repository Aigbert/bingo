//! The person's own schedule verbs (ADR-0094): `add` writes an entry they
//! asked for, and `confirm` makes one they did not ask for theirs. Both run
//! only on an asked line — what the kernel binds as the person's own input —
//! so a model, a mod, a drawn button or a correspondent cannot mint the
//! consent these entries carry.

use bingo_sdk::{CommandContext, CommandOutcome, ErrorCode, KernelError};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::entry::{Entry, Provenance};
use crate::render;
use crate::schedules::Schedules;

/// The verb that writes an entry the person asked for.
pub const ADD: &str = "add";
/// The verb that makes an entry the person's after they have read it.
pub const CONFIRM: &str = "confirm";

/// What separates the spec and mode from the text the entry will say.
const TEXT: &str = " -- ";
/// What introduces the mode, between the spec and the text.
const MODE: &str = " as ";

const USAGE: &str = "usage: /schedule add <spec> [as <mode>] -- <text>";

/// An `add` line, read: when, in which mode, and what to say.
#[derive(Debug, PartialEq, Eq)]
pub struct Added {
    pub spec: crate::Spec,
    pub mode: Option<String>,
    pub text: String,
}

/// `<spec> [as <mode>] -- <text>`. Pure, so the grammar has its own tests.
pub fn added(args: &str) -> Result<Added, String> {
    let (head, text) = args.split_once(TEXT).ok_or(USAGE)?;
    let text = text.trim();
    if text.is_empty() {
        return Err("a schedule with nothing to say would fire and do nothing".into());
    }
    let (spec, mode) = match head.rsplit_once(MODE) {
        Some((spec, mode)) => (spec, Some(mode.trim())),
        None => (head, None),
    };
    if mode.is_some_and(|mode| mode.is_empty() || mode.contains(char::is_whitespace)) {
        return Err(USAGE.into());
    }
    Ok(Added {
        spec: spec
            .trim()
            .parse()
            .map_err(|e: crate::SpecError| e.to_string())?,
        mode: mode.map(str::to_string),
        text: text.to_string(),
    })
}

/// Refuse anything but the person's own asked line.
fn persons_own(verb: &str) -> Result<(), KernelError> {
    match bingo_sdk::render::asked() {
        true => Ok(()),
        false => Err(KernelError::new(
            ErrorCode::PermissionDenied,
            format!("only the person's own /schedule {verb} is consent"),
        )),
    }
}

/// `/schedule add`: the entry, written as the person's.
pub fn add(
    schedules: &Schedules,
    args: &str,
    cx: &CommandContext,
) -> Result<CommandOutcome, KernelError> {
    persons_own(ADD)?;
    let added = added(args).map_err(invalid)?;
    let entry = Entry {
        id: schedules.store().mint(),
        spec: added.spec,
        text: added.text,
        cwd: cx.cwd.clone(),
        permission_mode: added.mode,
        enabled: true,
        created: Timestamp::now(),
        last_fired: None,
        by: Provenance::Person,
    };
    written(schedules, &entry, "scheduled")
}

/// `/schedule confirm <id>`: an entry the person has read, made theirs.
pub fn confirm(schedules: &Schedules, args: &str) -> Result<CommandOutcome, KernelError> {
    persons_own(CONFIRM)?;
    let shelf = schedules.store().load();
    let entry = crate::tools::find(&shelf, args.trim()).map_err(invalid)?;
    let confirmed = Entry {
        by: Provenance::Person,
        ..entry.clone()
    };
    written(schedules, &confirmed, "confirmed")
}

/// Write the entry down, ring the bell, and say exactly what now fires.
fn written(
    schedules: &Schedules,
    entry: &Entry,
    done: &str,
) -> Result<CommandOutcome, KernelError> {
    schedules
        .store()
        .save(entry)
        .map_err(|e| KernelError::new(ErrorCode::Internal, e.to_string()))?;
    schedules.changed();
    let mode = entry.permission_mode.as_deref().unwrap_or("the session's");
    Ok(CommandOutcome::Applied {
        message: Some(format!(
            "{done} {}: {}, next {}, mode {mode}: {}",
            entry.id,
            entry.spec,
            render::when(entry.next_fire(&TimeZone::system()).as_ref()),
            entry.text
        )),
    })
}

fn invalid(message: String) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_add_line_reads_its_spec_mode_and_text() {
        assert_eq!(
            added("every 30m -- /wake").expect("a line"),
            Added {
                spec: "every 30m".parse().expect("a spec"),
                mode: None,
                text: "/wake".into(),
            }
        );
        let read = added("daily at 09:00 as acceptEdits -- tidy -- the docs ").expect("a line");
        assert_eq!(read.spec, "daily at 09:00".parse().expect("a spec"));
        assert_eq!(read.mode.as_deref(), Some("acceptEdits"));
        assert_eq!(
            read.text, "tidy -- the docs",
            "only the first separator splits"
        );
    }

    #[test]
    fn an_add_line_without_its_parts_is_refused() {
        for wrong in [
            "every 30m",
            "every 30m -- ",
            "every 30m as -- x",
            "every 30m as two words -- x",
            "yearly -- x",
        ] {
            assert!(added(wrong).is_err(), "{wrong:?}");
        }
    }
}
