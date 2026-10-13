//! One bounded client belongs to the runtime; dropping fetch cancels its I/O.
use super::{
    scope::{LIMIT, cap},
    world::{Fetch, Fetched},
};
use std::collections::BTreeMap;

pub struct Http(reqwest::Client);
impl Http {
    pub fn new() -> Result<Self, String> {
        bingo_http::client(bingo_http::Bounds::CALL)
            .map(Self)
            .map_err(failed)
    }
    pub async fn fetch(&self, input: Fetch) -> Result<Fetched, String> {
        let url = url::Url::parse(&input.url).map_err(failed)?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err("http.fetch accepts only http and https URLs".into());
        }
        let method =
            reqwest::Method::from_bytes(input.method.as_deref().unwrap_or("GET").as_bytes())
                .map_err(failed)?;
        let mut request = self.0.request(method, url);
        for (name, value) in input.headers.unwrap_or_default() {
            request = request.header(name, value);
        }
        if let Some(body) = input.body {
            cap(body.len(), LIMIT).map_err(failed)?;
            request = request.body(body);
        }
        let mut response = request.send().await.map_err(failed)?;
        let status = response.status().as_u16();
        let headers: BTreeMap<_, _> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let mut bytes = Vec::new();
        let mut truncated = false;
        while let Some(chunk) = response.chunk().await.map_err(failed)? {
            let remaining = LIMIT - bytes.len();
            bytes.extend_from_slice(&chunk[..remaining.min(chunk.len())]);
            if chunk.len() > remaining {
                truncated = true;
                break;
            }
        }
        let notice = truncated.then(|| format!("response body was cut at the {LIMIT} byte limit"));
        Ok(Fetched {
            status,
            headers,
            body: String::from_utf8_lossy(&bytes).into_owned(),
            truncated,
            notice,
        })
    }
}
fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn peer(body: Vec<u8>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nX-World: local\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            let _ = socket.write_all(&body).await;
        });
        (url, task)
    }
    fn input(url: String) -> Fetch {
        Fetch {
            session: bingo_sdk::SessionId::mint(),
            url,
            method: None,
            headers: None,
            body: None,
        }
    }
    #[tokio::test]
    async fn local_fetch_answers_and_reports_a_cut_body() {
        for size in [5, LIMIT + 1] {
            let (url, task) = peer(vec![b'x'; size]).await;
            let response = Http::new().unwrap().fetch(input(url)).await.unwrap();
            assert_eq!(response.status, 200);
            assert_eq!(response.headers["x-world"], "local");
            assert_eq!(response.body.len(), size.min(LIMIT));
            assert_eq!(response.truncated, size > LIMIT);
            assert_eq!(response.notice.is_some(), size > LIMIT);
            task.await.unwrap();
        }
    }
    #[tokio::test]
    async fn dropping_fetch_closes_a_pending_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let http = Http::new().unwrap();
        let fetch = tokio::spawn(async move { http.fetch(input(url)).await });
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(socket.read_u8().await.unwrap());
        }
        fetch.abort();
        assert!(fetch.await.unwrap_err().is_cancelled());
        let mut tail = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            socket.read_to_end(&mut tail),
        )
        .await
        .expect("cancelled request closes socket")
        .unwrap();
    }
}
