use bingo_sdk::CancellationToken;
use std::sync::Mutex;

// The fixture serializes publication without keeping a second board state.
static OWNER: Mutex<Option<CancellationToken>> = Mutex::new(None);
pub struct Writer;

pub async fn acquire() -> Writer {
    loop {
        let waiting = {
            let mut owner = OWNER.lock().unwrap_or_else(|error| error.into_inner());
            if let Some(done) = owner.as_ref() {
                Some(done.clone())
            } else {
                *owner = Some(CancellationToken::new());
                None
            }
        };
        match waiting {
            Some(done) => done.cancelled().await,
            None => return Writer,
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        let done = OWNER
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(done) = done {
            done.cancel();
        }
    }
}
