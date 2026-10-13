//! Each offered specification may be rewritten or removed, never added.
use super::*;

pub(crate) async fn tools(
    specs: Vec<ToolSpec>,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<Vec<ToolSpec>, KernelError> {
    let mut described = Vec::new();
    for spec in specs {
        let answer = json::dispatch(
            catalogue::TOOL_DESCRIBE,
            links,
            cx,
            spec,
            Next::new(|spec| Box::pin(async { Ok(Some(spec)) })),
            report.clone(),
        )
        .await?;
        if let Some(spec) = answer {
            described.push(spec);
        }
    }
    Ok(described)
}
