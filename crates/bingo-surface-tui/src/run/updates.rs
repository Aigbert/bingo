//! The one thing this surface asks of the network at start: whether a newer
//! release than this build is out (M63, ADR-0043).
//!
//! It is spawned, never awaited: a start pays nothing for it, the answer
//! arrives as a reply like any other, and every way it can fail is silence
//! and a debug line. Only a run with a welcome box to say it in asks at all.

use std::path::PathBuf;

use jiff::Timestamp;
use tokio::sync::mpsc;

use super::Reply;

/// This build's version, which is what a release is tagged.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Ask, off the loop's thread, and mail back a version worth saying.
pub(super) fn spawn(replies: mpsc::Sender<Reply>, data_dir: PathBuf) {
    tokio::spawn(async move {
        // The start after an update that could not remove what it moved
        // aside: Windows holds a running image open, so this is where the
        // last one is finally swept.
        if let Ok(running) = bingo_update::install::running() {
            bingo_update::install::sweep(&running);
        }
        let found = bingo_update::check(CURRENT, &data_dir, Timestamp::now(), fetch).await;
        if let Some(version) = found {
            let _ = replies.send(Reply::Update(version)).await;
        }
    });
}

/// One GET with a clock on it. The library reaches no network of its own, so
/// this is where the client is: five seconds, and this build's own name.
async fn fetch(url: String) -> Result<String, String> {
    let client = bingo_http::client(bingo_http::Bounds {
        connect: bingo_http::CONNECT_TIMEOUT,
        call: Some(bingo_update::api::TIMEOUT),
    })
    .map_err(|e| e.to_string())?;
    client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            bingo_update::api::user_agent(CURRENT),
        )
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}
