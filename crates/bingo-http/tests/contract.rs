use bingo_http::{Bounds, CONNECT_TIMEOUT, FRAME, IDLE_TIMEOUT, client};
use std::time::Duration;

#[test]
fn the_bounds_preserve_the_transport_contract() {
    assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(20));
    assert_eq!(IDLE_TIMEOUT, Duration::from_secs(300));
    assert_eq!(FRAME, 64 * 1024);
    assert_eq!(Bounds::STREAMED.connect, CONNECT_TIMEOUT);
    assert_eq!(Bounds::STREAMED.call, None);
    assert_eq!(Bounds::CALL.connect, Duration::from_secs(10));
    assert_eq!(Bounds::CALL.call, Some(Duration::from_secs(30)));
    assert!(client(Bounds::STREAMED).is_ok());
    assert!(client(Bounds::CALL).is_ok());
}
