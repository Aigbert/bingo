use bingo_http::round_trip;
use bingo_sdk::ProviderError;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    time::Instant,
};

/// Small enough that the kernel cannot swallow the request whole, so the
/// peer's pace is the upload's pace.
const PEER_BUFFER: u32 = 64 * 1024;

/// What the slow peer takes in one go — several frames' worth, so it is
/// the peer that paces the clock and not the frame size.
const SIP: usize = 256 * 1024;

/// Bigger than the buffers on either side, so the body is still going out
/// while the assertions watch it: 32 sips, and 32 gaps to go with them.
const UPLOAD: usize = 8 * 1024 * 1024;

/// The gap the slow peer leaves between two sips, and the silence a test
/// allows. Twenty-five gaps: a machine that stalls for twenty-four of them
/// still passes, and the upload still outlasts the silence it is given.
const GAP: Duration = Duration::from_millis(20);
const IDLE: Duration = Duration::from_millis(500);

const ANSWER: &[u8] = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}";

/// A listener on the loopback, and the url that reaches it.
fn peer() -> (TcpListener, String) {
    let socket = TcpSocket::new_v4().expect("a socket");
    socket
        .set_recv_buffer_size(PEER_BUFFER)
        .expect("a small receive buffer");
    socket
        .bind("127.0.0.1:0".parse().expect("a loopback address"))
        .expect("a port");
    let listener = socket.listen(1).expect("a listener");
    let address = listener.local_addr().expect("the port it took");
    (listener, format!("http://{address}"))
}

fn client() -> reqwest::Client {
    bingo_http::client(bingo_http::Bounds::STREAMED).expect("a bounded client")
}

fn uploading(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    client.post(url).body("x".repeat(UPLOAD))
}

/// The request head, one byte at a time so that none of the body is
/// swallowed with it.
async fn head(socket: &mut TcpStream) -> String {
    let mut seen = Vec::new();
    let mut byte = [0u8; 1];
    while !seen.ends_with(b"\r\n\r\n") && socket.read_exact(&mut byte).await.is_ok() {
        seen.push(byte[0]);
    }
    String::from_utf8_lossy(&seen).into_owned()
}

fn declared_length(head: &str) -> usize {
    head.to_lowercase()
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length:")
                .map(str::trim)
                .map(str::to_string)
        })
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("no content length in {head:?}"))
}

/// Reads the request a sip at a time, `gap` apart, and answers only once
/// the whole of it has arrived. Hands back the head it read. The sip is
/// exact rather than whatever happens to have landed, so the drain rate is
/// the peer's own and not the kernel's.
async fn a_trickle(listener: TcpListener, gap: Duration) -> String {
    let (mut socket, _) = listener.accept().await.expect("a connection");
    let head = head(&mut socket).await;
    let mut left = declared_length(&head);
    let mut sip = vec![0u8; SIP];
    while left > 0 {
        tokio::time::sleep(gap).await;
        let take = SIP.min(left);
        if socket.read_exact(&mut sip[..take]).await.is_err() {
            break;
        }
        left -= take;
    }
    socket.write_all(ANSWER).await.expect("the answer");
    head
}

/// Takes one mouthful and never reads again, holding the connection open
/// so that nothing but the guard can end the request.
async fn a_deaf_peer(listener: TcpListener) {
    let (mut socket, _) = listener.accept().await.expect("a connection");
    head(&mut socket).await;
    let mut sip = vec![0u8; SIP];
    let _ = socket.read(&mut sip).await;
    std::future::pending::<()>().await;
}

/// Reads everything and answers nothing.
async fn a_mute_peer(listener: TcpListener) {
    let (mut socket, _) = listener.accept().await.expect("a connection");
    let mut sink = Vec::new();
    let _ = socket.read_to_end(&mut sink).await;
}

/// The 8.8 MB replay of ADR-0056 in miniature: the peer takes longer than
/// the whole idle to read the body, and the request still lands.
#[tokio::test]
async fn a_slow_upload_is_not_a_silence() {
    let (listener, url) = peer();
    let reading = tokio::spawn(a_trickle(listener, GAP));
    let client = client();
    let started = Instant::now();
    let sent = round_trip(uploading(&client, &url), IDLE).await;
    let took = started.elapsed();
    assert!(
        sent.is_ok(),
        "a moving upload is never cut: {:?}",
        sent.err()
    );
    assert!(took > IDLE, "it outlived its own idle: {took:?}");
    reading.await.expect("the peer");
}

/// A peer that stops reading has gone quiet: the request ends as the
/// `Timeout` the turn loop's ladder knows, and not before the idle.
#[tokio::test]
async fn a_peer_that_stops_reading_ends_the_request() {
    let (listener, url) = peer();
    let deaf = tokio::spawn(a_deaf_peer(listener));
    let client = client();
    let started = Instant::now();
    let sent = round_trip(uploading(&client, &url), IDLE).await;
    let took = started.elapsed();
    assert_eq!(sent.err(), Some(ProviderError::Timeout));
    assert!(took >= IDLE, "a silence is not one until it has lasted");
    deaf.abort();
}

/// A server that takes the whole request and then says nothing is the
/// silence the guard was always for: a headless run must not hang on it.
#[tokio::test]
async fn a_server_that_accepts_and_says_nothing_times_out() {
    let (listener, url) = peer();
    let mute = tokio::spawn(a_mute_peer(listener));
    let client = client();
    let started = Instant::now();
    let asking = client.post(url.as_str()).body("hello");
    assert_eq!(
        round_trip(asking, IDLE).await.err(),
        Some(ProviderError::Timeout)
    );
    assert!(started.elapsed() >= IDLE);
    mute.abort();
}

/// An exact size hint is what keeps `Content-Length` on the request; a
/// chunked body is a different request, and a relay may refuse it.
#[tokio::test]
async fn the_request_head_carries_its_length_and_is_never_chunked() {
    let (listener, url) = peer();
    let reading = tokio::spawn(a_trickle(listener, Duration::ZERO));
    let client = client();
    round_trip(uploading(&client, &url), IDLE * 10)
        .await
        .expect("the request lands");
    let head = reading.await.expect("the peer").to_lowercase();
    assert!(
        head.contains(&format!("content-length: {UPLOAD}")),
        "the head declares the whole body: {head}"
    );
    assert!(
        !head.contains("transfer-encoding"),
        "and chunks nothing: {head}"
    );
}
