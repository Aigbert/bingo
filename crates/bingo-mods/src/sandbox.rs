//! The engine compiler stays internal; no dynamic-code constructor reaches a mod.
use rquickjs::{AsyncContext, AsyncRuntime, context::intrinsic};

pub async fn context(runtime: &AsyncRuntime) -> rquickjs::Result<AsyncContext> {
    let context = AsyncContext::custom::<(
        intrinsic::Eval,
        intrinsic::Json,
        intrinsic::MapSet,
        intrinsic::Promise,
        intrinsic::Proxy,
        intrinsic::RegExp,
        intrinsic::TypedArrays,
    )>(runtime)
    .await?;
    context.with(|ctx| {
        crate::globals::install(&ctx)?;
        ctx.eval::<(), _>(r#"
            (() => {
                const denied = () => { throw new TypeError('dynamic code is not available in a mod'); };
                for (const fn of [function(){}, async function(){}, function*(){}, async function*(){}]) {
                    Object.defineProperty(Object.getPrototypeOf(fn), 'constructor', { value: denied, writable: false, configurable: false });
                }
                Object.defineProperty(globalThis, 'eval', { value: denied, writable: false, configurable: false });
                Object.defineProperty(globalThis, 'Function', { value: denied, writable: false, configurable: false });
            })();
        "#)
    }).await?;
    Ok(context)
}
