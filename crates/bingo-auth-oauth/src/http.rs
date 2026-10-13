//! The bounded client for small OAuth token and discovery requests.

/// A shared construction point for token and discovery clients.
pub fn client() -> reqwest::Client {
    reqwest::Client::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthError, Challenge, CredentialStore, Entry, TokenSource};
    use std::{sync::Arc, time::Duration};
    use tokio::io::AsyncReadExt;

    async fn silent() -> (
        String,
        tokio::sync::oneshot::Receiver<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (sent, heard) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut byte = [0];
            socket.read_exact(&mut byte).await.unwrap();
            let _ = sent.send(());
            std::future::pending::<()>().await;
        });
        (url, heard, task)
    }

    async fn expires<T: std::fmt::Debug>(
        future: impl Future<Output = Result<T, AuthError>>,
        heard: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut running = Box::pin(future);
        tokio::select! {
            result = &mut running => panic!("answered before the silent server: {result:?}"),
            _ = heard => {},
        }
        tokio::time::pause();
        let result = tokio::time::timeout(Duration::from_secs(31), &mut running).await;
        tokio::time::resume();
        assert!(matches!(
            result.expect("OAuth requests must have a total deadline"),
            Err(AuthError::Timeout)
        ));
    }

    #[tokio::test]
    async fn deadline_bounds_a_codex_token_refresh() {
        let (url, heard, server) = silent().await;
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(CredentialStore::new(dir.path().into()));
        store
            .write(
                "codex",
                Entry::OAuth {
                    access: "old".into(),
                    refresh: "refresh".into(),
                    expires: 1,
                    account_id: None,
                },
            )
            .unwrap();
        let source = TokenSource::new("codex", crate::issuer::tests::issuer(&url), store, client());
        expires(source.access_token(), heard).await;
        server.abort();
    }

    #[tokio::test]
    async fn deadline_bounds_mcp_auth_discovery() {
        let (url, heard, server) = silent().await;
        expires(
            crate::discover::discover(&client(), &url, &Challenge::default()),
            heard,
        )
        .await;
        server.abort();
    }
}
