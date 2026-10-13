//! The scheduler's original on-disk protocol over the shared OS claim.
use std::path::Path;

pub use bingo_claim::ClaimError;

/// The on-disk protocol marker, not evidence that any runner is alive.
pub const MARKER: &str = "bingo-schedule-runner-v1\n";
const PROTOCOL: bingo_claim::Protocol<'_> = bingo_claim::Protocol {
    file: "runner.lock",
    marker: MARKER,
};

#[derive(Debug)]
pub struct Claim(bingo_claim::Claim);

impl Claim {
    pub fn take(dir: &Path) -> Result<Self, ClaimError> {
        bingo_claim::Claim::take(dir, PROTOCOL).map(Self)
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

pub fn probe(dir: &Path) -> Result<bool, ClaimError> {
    bingo_claim::probe(dir, PROTOCOL)
}

pub fn holder(dir: &Path, held: bool) -> String {
    bingo_claim::holder(dir, PROTOCOL, held)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheduler_protocol_is_unchanged() {
        assert_eq!(PROTOCOL.file, "runner.lock");
        assert_eq!(PROTOCOL.marker, "bingo-schedule-runner-v1\n");
        let home = tempfile::tempdir().unwrap();
        let claim = Claim::take(home.path()).unwrap();
        let path = claim.path().to_path_buf();
        drop(claim);
        assert_eq!(std::fs::read(path).unwrap(), PROTOCOL.marker.as_bytes());
    }
}
