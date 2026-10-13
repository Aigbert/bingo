//! One process owns one app (ADR-0016 §5).
//!
//! Feishu's long connection is cluster-mode — a random client gets each event
//! — and Telegram's polling evicts whoever polled last. A second bingo on the
//! same credential is therefore not a conflict anybody sees: it is half the
//! messages going somewhere else, silently. So the claim is taken before the
//! adapter starts, and a claim that cannot be taken is a refusal with the
//! remedy in it, never a degraded start.

use std::path::Path;

use crate::error::ChannelError;

/// Everything this surface keeps on disk lives under it.
pub(crate) use bingo_channel_adapter::DIRECTORY;

/// A credential this process holds for as long as the value lives.
#[derive(Debug)]
pub struct Claim(bingo_claim::Claim);

/// Identifies a permanent channel claim, never evidence of a live holder.
pub const MARKER: &str = "bingo-channel-claim-v1\n";

impl Claim {
    pub fn take(data_dir: &Path, adapter: &str, credential: &str) -> Result<Self, ChannelError> {
        let directory = data_dir.join(DIRECTORY);
        let file = format!("{}.lock", slug(&format!("{adapter}-{credential}")));
        let protocol = bingo_claim::Protocol {
            file: &file,
            marker: MARKER,
        };
        bingo_claim::Claim::take(&directory, protocol)
            .map(Self)
            .map_err(|error| refusal(adapter, error))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

fn refusal(adapter: &str, error: bingo_claim::ClaimError) -> ChannelError {
    match error {
        bingo_claim::ClaimError::WouldBlock { path } => taken(adapter, &path),
        bingo_claim::ClaimError::Legacy { path } => ChannelError::Refused(format!(
            "legacy or unrecognized channel claim ({}); stop every bingo using this data dir, \
             inspect and remove that file, then restart",
            path.display()
        )),
        error => ChannelError::Refused(error.to_string()),
    }
}

/// The words a taken claim refuses with; [`held_elsewhere`] answers from
/// them, so the two cannot drift apart.
const TAKEN: &str = "another bingo already runs the ";

fn taken(adapter: &str, path: &Path) -> ChannelError {
    ChannelError::Refused(format!(
        "{TAKEN}{adapter} channel for this app: a second one \
         would take half of its events and neither would know. The OS lock on {} is held; \
         stop the other listener before retrying. Never remove this permanent file.",
        path.display()
    ))
}

/// Whether an error is this module's "someone else already listens". A
/// dedicated listener reads that as the refusal it is; a *beside* listener
/// reads it as the design working — the lock exists so exactly one process
/// takes the app's events, and the gateway is usually the one that has it.
pub fn held_elsewhere(error: &bingo_sdk::KernelError) -> bool {
    error.message.starts_with(TAKEN)
}

/// A file name from an identifier that may hold anything: only what is safe
/// on every filesystem survives, and the rest becomes `_`.
fn slug(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_second_claim_on_one_credential_is_refused_loudly() {
        let home = tempfile::tempdir().expect("a temporary directory");
        let first = Claim::take(home.path(), "feishu", "cli_a1").expect("the first claim");
        let second = Claim::take(home.path(), "feishu", "cli_a1");
        let Err(ChannelError::Refused(message)) = second else {
            panic!("expected a refusal, got {second:?}");
        };
        assert!(message.contains("another bingo already runs"), "{message}");
        assert!(
            message.contains(&first.path().display().to_string()),
            "the remedy names the file: {message}"
        );
    }

    #[test]
    fn legacy_files_are_refused_with_a_remedy_and_never_changed() {
        for bytes in [b"4242".as_slice(), b""] {
            let home = tempfile::tempdir().unwrap();
            let dir = home.path().join(DIRECTORY);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("feishu-cli_a1.lock");
            std::fs::write(&path, bytes).unwrap();
            let error = Claim::take(home.path(), "feishu", "cli_a1").unwrap_err();
            let text = error.to_string();
            assert!(text.contains("legacy"), "{text}");
            assert!(text.contains("stop every bingo"), "{text}");
            assert!(text.contains(&path.display().to_string()), "{text}");
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn a_different_credential_is_a_different_claim() {
        let home = tempfile::tempdir().expect("a temporary directory");
        let _a = Claim::take(home.path(), "feishu", "cli_a1").expect("a");
        let _b = Claim::take(home.path(), "feishu", "cli_b2").expect("b");
        let _c = Claim::take(home.path(), "loopback", "cli_a1").expect("c");
    }

    #[test]
    fn a_claim_is_given_back_when_it_is_dropped() {
        let home = tempfile::tempdir().expect("a temporary directory");
        let path = {
            let claim = Claim::take(home.path(), "feishu", "cli_a1").expect("a claim");
            claim.path().to_path_buf()
        };
        assert!(path.exists(), "release keeps the permanent claim inode");
        Claim::take(home.path(), "feishu", "cli_a1").expect("the next process may have it");
    }

    #[test]
    fn a_credential_with_a_path_in_it_cannot_escape_the_directory() {
        let home = tempfile::tempdir().expect("a temporary directory");
        let claim = Claim::take(home.path(), "loopback", "../../etc/passwd").expect("a claim");
        assert_eq!(
            claim.path().parent(),
            Some(home.path().join(DIRECTORY).as_path())
        );
    }
}
