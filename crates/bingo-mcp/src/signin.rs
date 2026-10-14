//! Cancelling a sign-in clears its activity even when the command future is dropped.
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
pub struct Signing {
    pub(crate) active: Arc<AtomicBool>,
    pub(crate) manager: Weak<crate::Manager>,
}
impl Drop for Signing {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        if let Some(manager) = self.manager.upgrade() {
            manager.changed();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn dropping_a_sign_in_restores_the_underlying_connection_state() {
        let servers =
            serde_json::from_value(serde_json::json!({"local":{"command":"unused"}})).unwrap();
        let manager = Arc::new(crate::Manager::new(
            bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
            servers,
            &[],
            std::env::temp_dir(),
        ));
        let signing = manager.signing_in("local").await.unwrap();
        assert_eq!(
            manager.rows().await[0].status,
            bingo_services::mcp::Status::SigningIn
        );
        assert!(manager.signing_in("local").await.is_none());
        drop(signing);
        assert_eq!(
            manager.rows().await[0].status,
            bingo_services::mcp::Status::Connecting
        );
    }
}
