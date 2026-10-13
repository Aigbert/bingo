#![allow(clippy::unwrap_used)]
use bingo_mods::sandbox;
use rquickjs::{AsyncRuntime, Module};

#[tokio::test]
async fn trusted_module_loading_survives_without_dynamic_code() {
    let runtime = AsyncRuntime::new().unwrap();
    let context = sandbox::context(&runtime).await.unwrap();
    context.async_with(async |ctx| {
        let promise = Module::evaluate(ctx.clone(), "test.js", "export const n = 7;").unwrap();
        promise.into_future::<()>().await.unwrap();
        for source in [
            "eval('1')", "new Function('return 1')()",
            "(function(){}).constructor('return 1')()",
            "(async function(){}).constructor('return 1')()",
            "(function*(){}).constructor('return 1')()",
            "(async function*(){}).constructor('return 1')()",
            "require('fs')", "process.cwd()", "setTimeout(() => {}, 0)",
        ] {
            assert!(ctx.eval::<rquickjs::Value, _>(source).is_err(), "escaped with {source}");
            let _ = ctx.catch();
        }
    }).await;
}

#[tokio::test]
async fn globals_are_available_and_encode_real_bytes() {
    let runtime = AsyncRuntime::new().unwrap();
    let context = sandbox::context(&runtime).await.unwrap();
    context.async_with(async |ctx| {
        let ok: bool = ctx.eval(r#"
            const encoded = new TextEncoder().encode('hello 🌍');
            const controller = new AbortController();
            let aborted = false;
            controller.signal.addEventListener('abort', () => aborted = true);
            controller.abort('stop');
            new TextDecoder().decode(encoded) === 'hello 🌍'
                && controller.signal.aborted && aborted
                && new URL('../ok', 'https://example.com/a/b').href === 'https://example.com/ok'
                && /^[0-9a-f-]{36}$/.test(crypto.randomUUID())
                && typeof performance.now() === 'number'
                && h('Text', { color: 'red' }, 'hello').children[0] === 'hello';
        "#).unwrap();
        assert!(ok);
        let promise = ctx.eval_promise("const hash = await crypto.subtle.digest('SHA-256', new TextEncoder().encode('abc')); Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2,'0')).join('')").unwrap();
        assert_eq!(promise.into_future::<String>().await.unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }).await;
}
