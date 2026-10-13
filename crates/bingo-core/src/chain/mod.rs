//! Ordered dispatch, with the kernel as its final link.
use bingo_sdk::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct Link {
    pub external: Option<String>,
    pub interceptor: Arc<dyn Interceptor>,
    pub tier: Tier,
}

mod tier;
pub use tier::{Act, admits, tightens};
