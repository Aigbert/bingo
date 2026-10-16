//! Read a refused response before its provider classifies it.
use bingo_sdk::ProviderError;

pub fn header(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

pub async fn refusal(
    response: reqwest::Response,
    classify: fn(u16, &str, Option<&str>) -> ProviderError,
) -> ProviderError {
    let status = response.status().as_u16();
    let retry_after = header(&response, "retry-after");
    let body = response.text().await.unwrap_or_default();
    classify(status, &body, retry_after.as_deref())
}
