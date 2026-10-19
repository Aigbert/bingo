//! The schedule owner a scene's world keeps: the prototype's three schedules
//! and the `/schedule` verbs bingo-schedule answers them with.
use super::kernel::Kernel;
use crate::test_support::ember::world::rows;
use bingo_sdk::*;
use bingo_services::schedule::{self, Listing, Pending, Row};
use std::sync::{Arc, Mutex};

pub(super) fn register(kernel: &Kernel) {
    let owner = Arc::new(Owner {
        switched: Mutex::default(),
        since: jiff::Timestamp::now(),
    });
    kernel.add(Arc::new(Command(owner.clone())));
    let service: Arc<dyn schedule::Service> = owner;
    super::builtins::serve(kernel, schedule::KEY.name(), service);
}

/// Which of the world's schedules the person paused or resumed, and the
/// moment their next fires were set from.
struct Owner {
    switched: Mutex<Vec<(String, bool)>>,
    since: jiff::Timestamp,
}

impl Owner {
    fn enabled(&self, id: &str, on: bool) -> bool {
        self.switched
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(switched, _)| switched == id)
            .map_or(on, |(_, enabled)| *enabled)
    }

    fn switch(&self, id: String, enabled: bool) -> Result<String, KernelError> {
        let listing = listing(self);
        let Some(row) = listing.entries.iter().find(|row| row.id == id) else {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "no such schedule",
            ));
        };
        let said = format!(
            "{} · {}",
            if enabled { "resumed" } else { "paused" },
            row.text
        );
        self.switched.lock().unwrap().push((id, enabled));
        Ok(said)
    }
}

/// The schedules as bingo-schedule lists them: the same next fires on every
/// read. The drawing reads the host's own clock and zone, so the fires are
/// set against that clock when the scene begins.
fn listing(owner: &Owner) -> Listing {
    let now = owner.since;
    let zone = jiff::tz::TimeZone::system();
    let entries = rows("scheds")
        .into_iter()
        .enumerate()
        .map(|(at, row)| {
            let id = at.to_string();
            Row {
                enabled: owner.enabled(&id, row["on"].as_bool().unwrap()),
                id,
                spec: row["when"].as_str().unwrap().into(),
                text: row["what"].as_str().unwrap().into(),
                next_fire: next(row["next"].as_str().unwrap(), now, &zone),
                by: "you".into(),
                mode: None,
                session: None,
            }
        })
        .collect();
    Listing {
        entries,
        ..Default::default()
    }
}

/// `in 12m` with the half minute the drawing's floor leaves room for, and
/// `tomorrow 09:00` in the zone the drawing reads.
fn next(word: &str, now: jiff::Timestamp, zone: &jiff::tz::TimeZone) -> Option<String> {
    match word {
        "in 12m" => Some((now + jiff::SignedDuration::from_secs(12 * 60 + 30)).to_string()),
        "tomorrow 09:00" => {
            let today = now.to_zoned(zone.clone()).date();
            let tomorrow = today.checked_add(jiff::Span::new().days(1)).unwrap();
            let at = tomorrow.at(9, 0, 0, 0).to_zoned(zone.clone()).unwrap();
            Some(at.timestamp().to_string())
        }
        _ => None,
    }
}

#[async_trait::async_trait]
impl schedule::Service for Owner {
    async fn list(&self) -> Result<Listing, KernelError> {
        Ok(listing(self))
    }
    async fn pending(&self, _: SessionId) -> Result<Option<Pending>, KernelError> {
        Ok(None)
    }
    async fn pause(&self, id: String) -> Result<String, KernelError> {
        self.switch(id, false)
    }
    async fn resume(&self, id: String) -> Result<String, KernelError> {
        self.switch(id, true)
    }
    async fn run(&self, _: String) -> Result<String, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "a scene runs no schedule",
        ))
    }
    async fn forget(&self, _: String) -> Result<String, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "a scene forgets no schedule",
        ))
    }
}

/// `/schedule pause|resume id`, handed to the owner as bingo-schedule's
/// command hands it.
struct Command(Arc<Owner>);

#[async_trait::async_trait]
impl bingo_sdk::Command for Command {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            keys: vec![],
            hidden: false,
            opens: true,
            name: "schedule".into(),
            aliases: vec![],
            hint: "prompts that run on a clock".into(),
            args: ArgSpec::None,
            instant: true,
            family: "session".into(),
        }
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        use schedule::Service;
        let mut words = args.split_whitespace();
        let message = match (words.next(), words.next()) {
            (Some("pause"), Some(id)) => self.0.pause(id.into()).await?,
            (Some("resume"), Some(id)) => self.0.resume(id.into()).await?,
            _ => return Ok(CommandOutcome::Applied { message: None }),
        };
        Ok(CommandOutcome::Applied {
            message: Some(message),
        })
    }
}
