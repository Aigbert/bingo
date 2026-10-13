use std::sync::Arc;
use futures::{FutureExt, StreamExt};
use serde_json::json;
use tokio::io::{AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio_util::codec::FramedRead;
use crate::{Handler, Peer, PeerError, Sender};
use crate::codec::{Message, MAX_LINE, lines};

struct Ignore;
impl Handler for Ignore {
    fn handle(&self, _: Message, _: Sender) -> futures::future::BoxFuture<'_, ()> {
        async {}.boxed()
    }
}

fn pair() -> (Arc<Peer>, FramedRead<ReadHalf<DuplexStream>, crate::codec::Lines>, WriteHalf<DuplexStream>) {
    let (ours, theirs) = tokio::io::duplex(4096);
    let (r, w) = tokio::io::split(ours);
    let (read, write) = tokio::io::split(theirs);
    (Arc::new(Peer::spawn(r, w, Arc::new(Ignore))), FramedRead::new(read, lines(MAX_LINE)), write)
}

#[tokio::test]
async fn out_of_order_responses_find_their_request() {
    let (peer, mut read, mut write) = pair();
    let one = tokio::spawn({ let peer = peer.clone(); async move { peer.request("one", json!({})).await } });
    let first: serde_json::Value = serde_json::from_str(&read.next().await.unwrap().unwrap()).unwrap();
    let two = tokio::spawn({ let peer = peer.clone(); async move { peer.request("two", json!({})).await } });
    let second: serde_json::Value = serde_json::from_str(&read.next().await.unwrap().unwrap()).unwrap();
    for (id, result) in [(second["id"].clone(), 2), (first["id"].clone(), 1)] {
        write.write_all(format!("{}\n", json!({"jsonrpc":"2.0", "id":id, "result":result})).as_bytes()).await.unwrap();
    }
    assert_eq!(one.await.unwrap().unwrap(), json!(1));
    assert_eq!(two.await.unwrap().unwrap(), json!(2));
}

#[tokio::test]
async fn a_dropped_request_removes_its_entry() {
    let (peer, mut read, _write) = pair();
    let task = tokio::spawn({ let peer = peer.clone(); async move { peer.request("held", json!({})).await } });
    read.next().await.unwrap().unwrap();
    assert_eq!(peer.pending_count(), 1);
    task.abort();
    let _ = task.await;
    assert_eq!(peer.pending_count(), 0);
}

#[tokio::test]
async fn closure_fails_every_pending_request() {
    let (peer, mut read, write) = pair();
    let mut requests = Vec::new();
    for _ in 0..3 {
        let peer = peer.clone();
        requests.push(tokio::spawn(async move { peer.request("held", json!({})).await }));
        read.next().await.unwrap().unwrap();
    }
    drop(write);
    drop(read);
    for task in requests { assert_eq!(task.await.unwrap(), Err(PeerError::Closed)); }
    assert_eq!(peer.request("late", json!({})).await, Err(PeerError::Closed));
}

#[tokio::test]
async fn an_overlong_line_ends_the_peer_without_a_newline() {
    let (peer, mut read, mut write) = pair();
    let task = tokio::spawn({ let peer = peer.clone(); async move { peer.request("held", json!({})).await } });
    read.next().await.unwrap().unwrap();
    let _ = write.write_all(&vec![b'x'; MAX_LINE + 1]).await;
    assert_eq!(task.await.unwrap(), Err(PeerError::LineTooLong));
    assert!(!peer.is_alive());
}
