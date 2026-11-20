use super::*;
use std::{fmt::Debug, future::Future, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const EXPECTED_CHANNEL_EVENTS: usize = 1;
const END_OF_STREAM: usize = 0;
const EMPTY_SOCKET_BUFFER: usize = 1;
const BURST_STREAM_NOT_CLOSED: &str = "incoming burst socket did not close";

#[derive(Clone, Copy, Debug)]
pub enum Phase {
    Open,
    Accept,
    FirstByte,
    ExactClose,
    ProducerClosed,
    CleanupSettled,
    SiblingExec,
}

pub fn diagnostic(fixture: &MemoryConnection, started: Instant, phase: Phase) -> String {
    format!(
        "phase={phase:?}; elapsed={:?}; hub_channels={:?}; server_outcome={}",
        started.elapsed(),
        fixture.filesystem.hub_peer_lifecycles(),
        fixture.filesystem.connection_outcome()
    )
}

pub async fn observe_phase<T, E: Debug>(
    fixture: &MemoryConnection,
    started: Instant,
    phase: Phase,
    operation: impl Future<Output = Result<T, E>>,
) -> T {
    let phase_started = Instant::now();
    match tokio::time::timeout_at(deadline(), operation).await {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => panic!(
            "{error:?}; phase_elapsed={:?}; {}",
            phase_started.elapsed(),
            diagnostic(fixture, started, phase)
        ),
        Err(error) => panic!(
            "{error}; phase_elapsed={:?}; {}",
            phase_started.elapsed(),
            diagnostic(fixture, started, phase)
        ),
    }
}

pub fn assert_exact_peer_close(
    fixture: &MemoryConnection,
    channel: russh::ChannelId,
    started: Instant,
) {
    assert_eq!(
        fixture.filesystem.hub_peer_lifecycles().get(&channel),
        Some(&super::super::memory_fs::HubPeerLifecycle {
            accepted: EXPECTED_CHANNEL_EVENTS,
            closes: EXPECTED_CHANNEL_EVENTS,
            forwarders_finished: EXPECTED_CHANNEL_EVENTS,
        }),
        "{}",
        diagnostic(fixture, started, Phase::ExactClose)
    );
}

pub async fn wait_peer_close(
    fixture: &MemoryConnection,
    channel: russh::ChannelId,
    started: Instant,
) {
    observe_phase(fixture, started, Phase::ExactClose, async {
        loop {
            if fixture
                .filesystem
                .hub_peer_lifecycles()
                .get(&channel)
                .is_some_and(|observed| {
                    observed.closes > NO_CLOSE_ATTEMPTS
                        && observed.forwarders_finished > INITIAL_COUNTER
                })
            {
                return Ok::<_, std::convert::Infallible>(());
            }
            tokio::time::sleep(SFTP_LIFECYCLE_POLL_INTERVAL).await;
        }
    })
    .await;
    assert_exact_peer_close(fixture, channel, started);
}

#[derive(Debug)]
pub struct BurstClosed {
    pub write_error: Option<io::ErrorKind>,
    pub read_error: Option<io::ErrorKind>,
}

pub async fn incoming_burst(mut socket: tokio::net::UnixStream) -> io::Result<BurstClosed> {
    let written = socket
        .write_all(&vec![PAYLOAD_FILL; LARGE_PAYLOAD_BYTES])
        .await;
    let write_error = match written {
        Ok(()) => None,
        Err(error) if closed(&error) => Some(error.kind()),
        Err(error) => return Err(error),
    };
    // Keep the producer socket alive after writing. Completion must come from
    // the exact peer channel closing, rather than producer drop or a timeout.
    let mut buffer = [u8::default(); EMPTY_SOCKET_BUFFER];
    let read_error = match socket.read(&mut buffer).await {
        Ok(END_OF_STREAM) => None,
        Err(error) if closed(&error) => Some(error.kind()),
        Err(error) => return Err(error),
        Ok(_) => return Err(io::Error::other(BURST_STREAM_NOT_CLOSED)),
    };
    Ok(BurstClosed {
        write_error,
        read_error,
    })
}

fn closed(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
    )
}
