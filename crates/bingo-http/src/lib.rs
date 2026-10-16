//! Bounded HTTP clients and metered event streams (ADR-0072).

pub mod metered;
mod redirects;
pub mod response;
pub mod sse;
pub mod stream;
mod transport;
pub use redirects::Redirects;

use bingo_sdk::{ModelEvent, ProviderError};
pub use metered::FRAME;
use std::time::Duration;
pub use transport::round_trip;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub connect: Duration,
    pub call: Option<Duration>,
    pub redirects: Redirects,
}

impl Bounds {
    pub const STREAMED: Self = Self {
        connect: CONNECT_TIMEOUT,
        call: None,
        redirects: Redirects::SameOrigin,
    };
    pub const CALL: Self = Self {
        connect: Duration::from_secs(10),
        call: Some(Duration::from_secs(30)),
        redirects: Redirects::SameOrigin,
    };
}

#[derive(Debug, thiserror::Error)]
#[error("cannot initialize the HTTP client: {0}")]
pub struct HttpError(#[from] pub reqwest::Error);

/// Every client has a connect bound; streaming clients have no total deadline.
#[allow(clippy::disallowed_methods)]
pub fn client(bounds: Bounds) -> Result<reqwest::Client, HttpError> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(bounds.connect)
        .redirect(bounds.redirects.policy());
    if let Some(call) = bounds.call {
        builder = builder.timeout(call);
    }
    Ok(builder.build()?)
}

/// The provider's vocabulary over a transport-independent SSE frame.
pub trait Decode: Send + 'static {
    fn decode(&mut self, event: &str, data: &str) -> Result<Vec<ModelEvent>, ProviderError>;
}
