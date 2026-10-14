//! Local request composition and its anchored context reading.
use super::ruler::Ruler;
use crate::context::estimate_tokens;
use bingo_sdk::*;

pub(super) fn request(ruler: &mut Ruler, request: &ModelRequest) -> ContextUsage {
    context(ruler, &request.system, &request.messages, &request.tools)
}

pub(super) fn context(
    ruler: &mut Ruler,
    system: &[SystemBlock],
    messages: &[Message],
    tools: &[ToolSpec],
) -> ContextUsage {
    let mut usage = ruler.measure(estimate_tokens(system, messages, tools));
    if !ruler.holds() {
        usage.parts = Some(bingo_sdk::tokens::composition(system, messages, tools));
    }
    usage
}
