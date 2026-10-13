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

use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct Report(Arc<Mutex<Vec<ItemBody>>>);
impl Report {
    fn notice(&self, link: &Link, point: &catalogue::Entry, reason: &str) {
        let name = link
            .external
            .as_deref()
            .unwrap_or_else(|| link.interceptor.id());
        self.push(ItemBody::Notice {
            level: Level::Warn,
            code: "INTERCEPTOR_SKIPPED".into(),
            text: format!(
                "{name} ({}): {reason}",
                serde_json::to_value(point)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_default()
            ),
        });
    }
    fn receipt(&self, link: &Link, call: Option<&str>) {
        self.push(ItemBody::Notice {
            level: Level::Info,
            code: "INTERCEPTOR_RECEIPT".into(),
            text: format!(
                "{} answered or rewrote {}",
                link.interceptor.id(),
                call.unwrap_or("tool.call")
            ),
        });
    }
    fn push(&self, item: ItemBody) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).push(item);
    }
    pub fn take(&self) -> Vec<ItemBody> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

pub(crate) trait Point: Send + Sync + 'static {
    type Input: Clone + PartialEq + Send + Sync;
    type Output: Clone + PartialEq + Send + Sync;
    const POINT: LinkPoint;
    fn entry(_: &Self::Input) -> &catalogue::Entry {
        Self::POINT
    }
    fn payload(input: &Self::Input) -> serde_json::Value {
        serde_json::json!({"tool": Self::tool(input)})
    }
    fn tool(_: &Self::Input) -> Option<&str> {
        None
    }
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>>;
    fn accepts(
        entry: &catalogue::Entry,
        tier: Tier,
        before: Option<&Self::Output>,
        after: &Self::Output,
    ) -> bool {
        match before {
            Some(before) => before == after || admits(tier, entry, Act::Rewrite),
            None => admits(tier, entry, Act::Answer),
        }
    }
    fn pinned(_before: &Self::Input, _after: &Self::Input) -> bool {
        true
    }
    fn needs_baseline(_: Tier, _: &Self::Output) -> bool {
        false
    }
}

type Pending<'a, O> = Shared<BoxFuture<'a, Result<O, KernelError>>>;
struct Continued<'a, O: Clone> {
    pending: Option<Pending<'a, O>>,
    invalid: bool,
    rewritten: bool,
    phase: tokio::sync::watch::Sender<bool>,
}
impl<O: Clone> Default for Continued<'_, O> {
    fn default() -> Self {
        Self {
            pending: None,
            invalid: false,
            rewritten: false,
            phase: tokio::sync::watch::channel(true).0,
        }
    }
}
type Continuation<'a, O> = Arc<Mutex<Continued<'a, O>>>;

type Kernel<'a, I, O> = Arc<Mutex<Option<Next<'a, I, O>>>>;

pub(crate) async fn run<'a, P: Point>(
    links: &'a [Link],
    kernel: Next<'a, P::Input, P::Output>,
    cx: &'a HookContext,
    input: P::Input,
    report: Report,
) -> Result<P::Output, KernelError> {
    if links.is_empty() {
        return kernel.run(input).await;
    }
    run_at::<P>(links, Arc::new(Mutex::new(Some(kernel))), cx, input, report).await
}

fn applies(matcher: &LinkMatcher, point: &catalogue::Entry, input: &serde_json::Value) -> bool {
    matcher.matches(point, input)
}

fn run_at<'a, P: Point>(
    links: &'a [Link],
    kernel: Kernel<'a, P::Input, P::Output>,
    cx: &'a HookContext,
    input: P::Input,
    report: Report,
) -> BoxFuture<'a, Result<P::Output, KernelError>> {
    Box::pin(async move {
        let Some((link, rest)) = links.split_first() else {
            let next = kernel.lock().unwrap_or_else(|e| e.into_inner()).take();
            return match next {
                Some(next) => next.run(input).await,
                None => Err(KernelError::new(
                    ErrorCode::Internal,
                    "dispatch continuation already consumed",
                )),
            };
        };
        if !applies(
            &link.interceptor.matcher(),
            P::entry(&input),
            &P::payload(&input),
        ) {
            return run_at::<P>(rest, kernel, cx, input, report).await;
        }
        invoke::<P>(link, rest, kernel, cx, input, report).await
    })
}

async fn invoke<'a, P: Point>(
    link: &'a Link,
    rest: &'a [Link],
    kernel: Kernel<'a, P::Input, P::Output>,
    cx: &'a HookContext,
    input: P::Input,
    report: Report,
) -> Result<P::Output, KernelError> {
    let entry = P::entry(&input).clone();
    let pending: Continuation<'a, P::Output> = Arc::new(Mutex::new(Continued::default()));
    let phase = pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .phase
        .subscribe();
    let next = continuation::<P>(
        link,
        rest,
        kernel.clone(),
        cx,
        input.clone(),
        report.clone(),
        pending.clone(),
    );
    let future = P::dispatch(link.interceptor.as_ref(), cx, input.clone(), next);
    let answer = if link.external.is_some() && !link.interceptor.owns_deadline() {
        phase_deadline(future, phase).await
    } else {
        future.await
    };
    let Continued {
        pending: called,
        invalid,
        rewritten,
        ..
    } = std::mem::take(&mut *pending.lock().unwrap_or_else(|e| e.into_inner()));
    let baseline = match called {
        Some(future) => Some(future.await),
        None if answer
            .as_ref()
            .is_ok_and(|output| P::needs_baseline(link.tier, output)) =>
        {
            Some(run_at::<P>(rest, kernel.clone(), cx, input.clone(), report.clone()).await)
        }
        None => None,
    };
    if rewritten && &entry == bingo_sdk::catalogue::TOOL_CALL {
        report.receipt(link, P::tool(&input));
    }
    if invalid {
        return match baseline {
            Some(result) => result,
            None => run_at::<P>(rest, kernel, cx, input, report).await,
        };
    }
    if let Ok(output) = &answer {
        let before = baseline.as_ref().and_then(|result| result.as_ref().ok());
        if P::accepts(&entry, link.tier, before, output) {
            if !rewritten && &entry == bingo_sdk::catalogue::TOOL_CALL && before != Some(output) {
                report.receipt(link, P::tool(&input));
            }
            return answer;
        }
        report.notice(link, &entry, "answer exceeds its trust tier");
    } else if let Err(error) = &answer {
        if error.code == ErrorCode::PermissionDenied && entry.acts.deny && baseline.is_none() {
            return answer;
        }
        report.notice(link, &entry, &error.message);
    }
    match baseline {
        Some(result) => result,
        None => run_at::<P>(rest, kernel, cx, input, report).await,
    }
}

fn continuation<'a, P: Point>(
    link: &'a Link,
    rest: &'a [Link],
    kernel: Kernel<'a, P::Input, P::Output>,
    cx: &'a HookContext,
    original: P::Input,
    report: Report,
    pending: Continuation<'a, P::Output>,
) -> Next<'a, P::Input, P::Output> {
    let entry = P::entry(&original).clone();
    let clock = pending
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .phase
        .clone();
    Next::at(link.tier, move |mut input| {
        let changed = input != original;
        let invalid =
            !P::pinned(&original, &input) || (changed && !admits(link.tier, &entry, Act::Rewrite));
        if invalid {
            report.notice(link, &entry, "input rewrite exceeds its authority");
            input = original;
        }
        let phase = pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .phase
            .clone();
        let future = run_at::<P>(rest, kernel, cx, input, report).shared();
        *pending.lock().unwrap_or_else(|e| e.into_inner()) = Continued {
            pending: Some(future.clone()),
            invalid,
            rewritten: changed && !invalid,
            phase,
        };
        Box::pin(future)
    })
    .tracked(clock)
}

pub(crate) mod compact;
pub(crate) mod compose;
pub(crate) mod model;
pub(crate) mod stop;
pub(crate) mod submit;
#[cfg(test)]
mod tests;
pub(crate) mod tool;

async fn phase_deadline<O>(
    future: BoxFuture<'_, Result<O, KernelError>>,
    mut phase: tokio::sync::watch::Receiver<bool>,
) -> Result<O, KernelError> {
    let timer = tokio::time::sleep(HOOK_DEADLINE);
    tokio::pin!(timer);
    tokio::pin!(future);
    loop {
        let active = *phase.borrow_and_update();
        tokio::select! {
            biased;
            result = &mut future => return result,
            changed = phase.changed() => {
                if changed.is_err() { return future.await; }
                timer.as_mut().reset(tokio::time::Instant::now() + HOOK_DEADLINE);
            }
            _ = &mut timer, if active => return Err(KernelError::new(ErrorCode::Internal, "hook deadline exceeded")),
        }
    }
}

pub(crate) mod json;

pub(crate) mod deliver;
pub(crate) mod describe;

pub(crate) mod complete;

pub(crate) mod command;

pub(crate) mod service;

pub(crate) mod autocomplete;

pub(crate) mod observe;

pub(crate) mod append;
pub(crate) mod config;

pub(crate) mod registration;
