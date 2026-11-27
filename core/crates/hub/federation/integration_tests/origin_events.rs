use super::{fixture, gate, source};
use std::sync::Arc;
use tokio::time::{Instant, timeout};
use yolo_hub_peer_protocol::FIRST_STREAM_SEQUENCE;
use yolo_hub_peer_rpc::RpcError;
use yolo_hub_terminal_protocol::{
    FIRST_OUTPUT_SEQUENCE, OutputChunk, OutputChunkFields, OutputCursor, PublicError, TerminalEvent,
};

#[path = "../test_support/origin_events.rs"]
mod support;
use gate::TEST_TIMEOUT;
use support::*;

#[tokio::test]
async fn origin_stream_delivers_signed_output_then_exit_and_releases_source_before_eof() {
    let fixture = fixture::NodeFixture::new().await;
    let (call, initial) = source::call(&fixture.node, &fixture.relay);
    let target = initial.fields().target;
    let (source, feed) = Source::new(initial);
    let mut updates = fixture
        .node
        .clone()
        .origin_stream(call.clone(), source)
        .unwrap();
    let mut sequence = FIRST_STREAM_SEQUENCE;
    let mut cursor = OutputCursor {
        target,
        after_sequence: u64::default(),
    };
    let output = TerminalEvent::Output {
        chunk: OutputChunk::new(OutputChunkFields {
            target,
            sequence: FIRST_OUTPUT_SEQUENCE,
            data: TERMINAL_OUTPUT.to_vec(),
        })
        .unwrap(),
    };
    feed.events.send(Ok(output.clone())).await.unwrap();
    assert_eq!(
        content(updates.as_mut(), &call, &mut sequence, &mut cursor)
            .await
            .unwrap(),
        output
    );
    assert_eq!(cursor.after_sequence, FIRST_OUTPUT_SEQUENCE);
    let exited = TerminalEvent::Exited {
        cursor,
        code: Some(SUCCESS_EXIT_CODE),
        signal: None,
    };
    feed.events.send(Ok(exited.clone())).await.unwrap();
    assert_eq!(
        content(updates.as_mut(), &call, &mut sequence, &mut cursor)
            .await
            .unwrap(),
        exited
    );
    timeout(TEST_TIMEOUT, feed.dropped).await.unwrap().unwrap();
    assert!(feed.events.is_closed());
    assert_eq!(
        timeout(TEST_TIMEOUT, updates.next()).await.unwrap(),
        Err(RpcError::Closed)
    );
    drop(updates);
    fixture.node.begin_shutdown();
    fixture
        .node
        .wait_retired(Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(Arc::strong_count(&fixture.node), 1);
    fixture
        .node
        .observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}

#[tokio::test]
async fn failed_terminal_source_reports_closed_then_eof_and_cannot_retain_the_generation() {
    let fixture = fixture::NodeFixture::new().await;
    let (call, initial) = source::call(&fixture.node, &fixture.relay);
    let mut cursor = OutputCursor {
        target: initial.fields().target,
        after_sequence: u64::default(),
    };
    let (source, feed) = Source::new(initial);
    let mut updates = fixture
        .node
        .clone()
        .origin_stream(call.clone(), source)
        .unwrap();
    let mut sequence = FIRST_STREAM_SEQUENCE;
    feed.events
        .send(Err(PublicError::Unreachable))
        .await
        .unwrap();
    assert_eq!(
        content(updates.as_mut(), &call, &mut sequence, &mut cursor).await,
        Err(RpcError::Closed)
    );
    timeout(TEST_TIMEOUT, feed.dropped).await.unwrap().unwrap();
    assert!(feed.events.is_closed());
    assert_eq!(
        timeout(TEST_TIMEOUT, updates.next()).await.unwrap(),
        Err(RpcError::Closed)
    );
    drop(updates);
    fixture.node.begin_shutdown();
    fixture
        .node
        .wait_retired(Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(Arc::strong_count(&fixture.node), 1);
    fixture
        .node
        .observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}

#[tokio::test]
async fn retired_origin_rejects_new_observation_and_releases_the_unstarted_source() {
    let fixture = fixture::NodeFixture::new().await;
    let (call, initial) = source::call(&fixture.node, &fixture.relay);
    let (source, feed) = Source::new(initial);
    fixture.node.begin_shutdown();
    assert!(fixture.node.clone().origin_stream(call, source).is_err());
    timeout(TEST_TIMEOUT, feed.dropped).await.unwrap().unwrap();
    assert!(feed.events.is_closed());
    fixture
        .node
        .wait_retired(Instant::now() + TEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(Arc::strong_count(&fixture.node), 1);
    fixture
        .node
        .observation_cache
        .wait_closed(Instant::now() + TEST_TIMEOUT)
        .await;
}
