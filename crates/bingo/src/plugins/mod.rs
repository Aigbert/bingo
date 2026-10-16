//! Plugin switches and the isolated author kit.
mod switch;
mod test;
mod types;
mod validate;

use crate::error::invalid;
use bingo_sdk::KernelError;
pub use switch::{Action, run};

pub async fn author(action: &Action) -> Option<Result<i32, KernelError>> {
    let result = match action {
        Action::Validate { dir } => validate::run(dir).await,
        Action::Test { dir, json } => test::run(dir, *json).await,
        Action::Types { dir } => types::run(dir).await,
        _ => return None,
    };
    Some(match result {
        Ok(()) => Ok(0),
        Err(error) => {
            eprintln!("{}", error.message);
            Ok(1)
        }
    })
}

/// Only this binary knows which session commands have a headless twin.
fn approval_hint(line: String, id: &str) -> String {
    match id.split_once(':') {
        Some((command @ ("plugins" | "mcp"), name)) => {
            format!("{line} (bingo {command} approve {name})")
        }
        _ => line,
    }
}

pub fn waiting_line(entry: &bingo_sdk::CatalogEntry) -> Option<String> {
    bingo_core::plugins::waiting_line(entry).map(|line| approval_hint(line, &entry.id))
}

fn waiting_reason(status: &bingo_sdk::PluginStatus) -> Option<String> {
    bingo_core::plugins::waiting_reason(&status.id, &status.state)
        .map(|line| approval_hint(line, &status.id))
}
