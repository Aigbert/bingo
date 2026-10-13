use super::*;

pub(crate) struct Compact;
impl Point for Compact {
    type Input = CompactInput;
    type Output = Result<Compaction, CompactError>;
    const POINT: LinkPoint = bingo_sdk::catalogue::SESSION_COMPACT;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.session_compact(cx, input, next))
    }
}

pub(crate) async fn kernel(
    input: CompactInput,
    compactor: &dyn Compactor,
    capabilities: &ModelCapabilities,
    provider: Arc<dyn Provider>,
    cancel: CancellationToken,
) -> Result<Result<Compaction, CompactError>, KernelError> {
    let cx = CompactContext {
        items: &input.items,
        usage: input.usage,
        capabilities,
        provider,
        request: &input.request,
        cancel,
        failures: input.failures,
        keep_budget: input.keep_budget,
    };
    Ok(compactor.compact(cx, input.reason).await)
}

pub(crate) async fn dispatch(
    links: &[Link],
    cx: &HookContext,
    input: CompactInput,
    compactor: &dyn Compactor,
    model: &crate::turn::ModelChoice,
    cancel: CancellationToken,
    report: Report,
) -> Result<Compaction, CompactError> {
    let provider = model.endpoint.provider().ok_or_else(|| {
        CompactError::from(KernelError::new(
            ErrorCode::InvalidInput,
            "an agent compacts its own context",
        ))
    })?;
    let kernel = Next::new(|input| {
        Box::pin(kernel(
            input,
            compactor,
            &model.capabilities,
            provider,
            cancel,
        ))
    });
    run::<Compact>(links, kernel, cx, input, report)
        .await
        .unwrap_or_else(|error| Err(error.into()))
}
