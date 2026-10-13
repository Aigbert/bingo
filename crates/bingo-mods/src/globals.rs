//! Native implementations with no ambient host access.
use rquickjs::{Ctx, Function, Object};

pub fn install(ctx: &Ctx<'_>) -> rquickjs::Result<()> {
    let native = Object::new(ctx.clone())?;
    native.set(
        "encode",
        Function::new(ctx.clone(), |text: String| text.into_bytes())?,
    )?;
    native.set("decode", Function::new(ctx.clone(), decode)?)?;
    native.set(
        "digest",
        Function::new(ctx.clone(), |bytes: Vec<u8>| {
            bingo_digest::digest(&bytes).to_vec()
        })?,
    )?;
    native.set("uuid", Function::new(ctx.clone(), uuid)?)?;
    native.set("url", Function::new(ctx.clone(), url)?)?;
    let started = std::time::Instant::now();
    native.set(
        "now",
        Function::new(ctx.clone(), move || {
            started.elapsed().as_secs_f64() * 1000.0
        })?,
    )?;
    ctx.globals().set("__bingo_native", native)?;
    ctx.eval::<(), _>(include_str!("globals.js"))
}

fn decode(ctx: Ctx<'_>, bytes: Vec<u8>, fatal: bool) -> rquickjs::Result<String> {
    if fatal {
        String::from_utf8(bytes)
            .map_err(|why| rquickjs::Exception::throw_type(&ctx, &why.to_string()))
    } else {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn uuid(ctx: Ctx<'_>) -> rquickjs::Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|why| rquickjs::Exception::throw_type(&ctx, &why.to_string()))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut text = String::with_capacity(36);
    for (index, byte) in bytes.into_iter().enumerate() {
        if [4, 6, 8, 10].contains(&index) {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(text)
}

fn url(ctx: Ctx<'_>, input: String, base: Option<String>) -> rquickjs::Result<String> {
    let parsed = match base {
        Some(base) => url::Url::parse(&base).and_then(|base| base.join(&input)),
        None => url::Url::parse(&input),
    }
    .map_err(|why| rquickjs::Exception::throw_type(&ctx, &why.to_string()))?;
    let data = serde_json::json!({
        "href": parsed.as_str(), "origin": parsed.origin().ascii_serialization(),
        "protocol": format!("{}:", parsed.scheme()), "hostname": parsed.host_str().unwrap_or_default(),
        "host": match parsed.port() { Some(port) => format!("{}:{port}", parsed.host_str().unwrap_or_default()), None => parsed.host_str().unwrap_or_default().to_string() },
        "port": parsed.port().map(|p| p.to_string()).unwrap_or_default(),
        "pathname": parsed.path(), "search": parsed.query().map(|q| format!("?{q}")).unwrap_or_default(),
        "hash": parsed.fragment().map(|f| format!("#{f}")).unwrap_or_default(),
        "username": parsed.username(), "password": parsed.password().unwrap_or_default(),
    });
    Ok(data.to_string())
}
