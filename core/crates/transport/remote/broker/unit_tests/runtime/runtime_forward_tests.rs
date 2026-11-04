use super::super::constants::*;
use super::super::runtime_forward::{forward, shutdown_local};
use super::support::connection_info;
use super::support_channel::{OpenStep, open_channel};
use super::support_channel_probe::ChannelProbe;
use crate::remote::{ChannelIdentity, ChannelPurpose};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BACKPRESSURE_CAPACITY: usize = 7;
const BIDIRECTIONAL_PAYLOAD_BYTES: usize = RUNTIME_FORWARD_BUFFER_BYTES * 2 + 31;
const BYTE_CHUNK_SIZE: usize = 1;
const LOCAL_PAYLOAD_SEED: u8 = 17;
const REMOTE_PAYLOAD_SEED: u8 = 91;
const STALLED_WRITE_TRIGGER: &[u8] = b"x";
const BLOCKED_LOCAL_WRITE_BYTES: usize = RUNTIME_FORWARD_BUFFER_BYTES * 64;

fn payload(length: usize, seed: u8) -> Vec<u8> {
    (0..length)
        .map(|index| seed.wrapping_add(index as u8))
        .collect()
}

fn identity() -> ChannelIdentity {
    ChannelIdentity {
        connection_id: connection_info().connection_id,
        channel_id: 1,
        purpose: ChannelPurpose::RuntimeControl,
    }
}

async fn write_chunks<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, bytes: &[u8]) {
    for chunk in bytes.chunks(BYTE_CHUNK_SIZE) {
        writer.write_all(chunk).await.unwrap();
    }
    writer.shutdown().await.unwrap();
}

#[tokio::test]
async fn simultaneous_backpressured_pumps_preserve_every_byte() {
    let local_sent = payload(BIDIRECTIONAL_PAYLOAD_BYTES, LOCAL_PAYLOAD_SEED);
    let remote_sent = payload(BIDIRECTIONAL_PAYLOAD_BYTES, REMOTE_PAYLOAD_SEED);
    let (local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, remote) = tokio::io::duplex(BACKPRESSURE_CAPACITY);
    let proxy = tokio::spawn(forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ));
    let (mut local_read, mut local_write) = local.into_split();
    let (mut remote_read, mut remote_write) = tokio::io::split(remote);
    let local_send = write_chunks(&mut local_write, &local_sent);
    let remote_send = write_chunks(&mut remote_write, &remote_sent);
    let mut at_local = Vec::new();
    let mut at_remote = Vec::new();
    let local_receive = local_read.read_to_end(&mut at_local);
    let remote_receive = remote_read.read_to_end(&mut at_remote);
    let (_, _, local_receive, remote_receive) =
        tokio::join!(local_send, remote_send, local_receive, remote_receive);
    local_receive.unwrap();
    remote_receive.unwrap();
    proxy.await.unwrap().unwrap();
    assert_eq!(at_local, remote_sent);
    assert_eq!(at_remote, local_sent);
}

#[tokio::test]
async fn local_half_close_drains_delayed_remote_tail() {
    const REQUEST: &[u8] = b"local request";
    const REMOTE_TAIL: &[u8] = b"delayed remote tail";
    let (mut local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, mut remote) = tokio::io::duplex(BACKPRESSURE_CAPACITY);
    let proxy = tokio::spawn(forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ));
    local.write_all(REQUEST).await.unwrap();
    local.shutdown().await.unwrap();
    let mut received = Vec::new();
    remote.read_to_end(&mut received).await.unwrap();
    assert_eq!(received, REQUEST);
    assert!(!proxy.is_finished());
    remote.write_all(REMOTE_TAIL).await.unwrap();
    remote.shutdown().await.unwrap();
    let mut tail = Vec::new();
    local.read_to_end(&mut tail).await.unwrap();
    proxy.await.unwrap().unwrap();
    assert_eq!(tail, REMOTE_TAIL);
}

#[tokio::test]
async fn remote_half_close_drains_delayed_local_tail() {
    const REMOTE_HEAD: &[u8] = b"remote head";
    const LOCAL_TAIL: &[u8] = b"delayed local tail";
    let (mut local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, mut remote) = tokio::io::duplex(BACKPRESSURE_CAPACITY);
    let proxy = tokio::spawn(forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ));
    remote.write_all(REMOTE_HEAD).await.unwrap();
    remote.shutdown().await.unwrap();
    let mut head = Vec::new();
    local.read_to_end(&mut head).await.unwrap();
    assert_eq!(head, REMOTE_HEAD);
    assert!(!proxy.is_finished());
    local.write_all(LOCAL_TAIL).await.unwrap();
    local.shutdown().await.unwrap();
    let mut tail = Vec::new();
    remote.read_to_end(&mut tail).await.unwrap();
    proxy.await.unwrap().unwrap();
    assert_eq!(tail, LOCAL_TAIL);
}

#[tokio::test(start_paused = true)]
async fn stalled_local_to_remote_write_has_directional_error() {
    let probe = ChannelProbe::new();
    let remote = open_channel(
        OpenStep::GatedHandshakeWrite,
        identity(),
        Arc::clone(&probe),
    );
    let (mut local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(forward(proxy_local.into(), remote, Vec::new()));
    local.write_all(STALLED_WRITE_TRIGGER).await.unwrap();
    local.shutdown().await.unwrap();
    probe.wait_write_pending(1).await;
    tokio::time::advance(RUNTIME_FORWARD_WRITE_TIMEOUT).await;
    let error = proxy.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RUNTIME_LOCAL_WRITE_TIMEOUT_MESSAGE);
    probe.wait_dropped(1).await;
}

#[tokio::test(start_paused = true)]
async fn stalled_shutdown_and_remote_tail_have_distinct_drain_error() {
    let probe = ChannelProbe::new();
    let remote = open_channel(OpenStep::ShutdownStall, identity(), Arc::clone(&probe));
    let (mut local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(forward(proxy_local.into(), remote, Vec::new()));
    local.shutdown().await.unwrap();
    probe.wait_shutdown_pending(1).await;
    tokio::time::advance(RUNTIME_REMOTE_SHUTDOWN_TIMEOUT).await;
    tokio::task::yield_now().await;
    tokio::time::advance(RUNTIME_TAIL_DRAIN_TIMEOUT).await;
    let error = proxy.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RUNTIME_REMOTE_TAIL_TIMEOUT_MESSAGE);
    probe.wait_dropped(1).await;
}

#[tokio::test(start_paused = true)]
async fn remote_eof_bounds_the_local_tail_drain() {
    let (mut local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, mut remote) = tokio::io::duplex(BACKPRESSURE_CAPACITY);
    let proxy = tokio::spawn(forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ));
    remote.shutdown().await.unwrap();
    let mut output = Vec::new();
    local.read_to_end(&mut output).await.unwrap();
    tokio::time::advance(RUNTIME_TAIL_DRAIN_TIMEOUT).await;
    let error = proxy.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RUNTIME_LOCAL_TAIL_TIMEOUT_MESSAGE);
}

#[tokio::test(start_paused = true)]
async fn stalled_remote_to_local_write_has_directional_error() {
    let (_local, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, mut remote) = tokio::io::duplex(BACKPRESSURE_CAPACITY);
    let proxy = tokio::spawn(forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ));
    let sender = tokio::spawn(async move {
        remote
            .write_all(&vec![0_u8; BLOCKED_LOCAL_WRITE_BYTES])
            .await
    });
    let error = proxy.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RUNTIME_REMOTE_WRITE_TIMEOUT_MESSAGE);
    sender.abort();
    let _ = sender.await;
}

#[tokio::test(start_paused = true)]
async fn stalled_local_shutdown_has_distinct_error() {
    let probe = ChannelProbe::new();
    let mut channel = open_channel(OpenStep::ShutdownStall, identity(), Arc::clone(&probe));
    let shutdown = tokio::spawn(async move { shutdown_local(&mut channel).await });
    probe.wait_shutdown_pending(1).await;
    tokio::time::advance(RUNTIME_LOCAL_SHUTDOWN_TIMEOUT).await;
    let error = shutdown.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), RUNTIME_LOCAL_SHUTDOWN_TIMEOUT_MESSAGE);
    probe.wait_dropped(1).await;
}
