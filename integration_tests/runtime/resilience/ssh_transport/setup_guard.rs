use super::super::constants::{
    DETACHED_HOST_PROBE_POLL_INTERVAL, DETACHED_HOST_PUBLICATION_TIMEOUT,
};
use super::super::runtime_guard::DetachedRuntimeGuard;
use anyhow::{Context, Result, anyhow, ensure};
use std::path::Path;
use tokio::net::UnixStream;
use tokio::time::Instant;
use yolo_client::{Client, ClientError};
use yolo_process_test_support::ProcessIdentity;
use yolo_runtime::model::HostStatus;
use yolo_runtime::paths::Paths;
use yolo_runtime::protocol::ProtocolError;

const DETACHED_PEER_UID_CHANGED_DETAIL: &str = "detached Host peer belongs to another user";
const DETACHED_PEER_PID_MISSING_DETAIL: &str = "detached Host peer PID is unavailable";
const DETACHED_PEER_PID_INVALID_DETAIL: &str = "detached Host peer PID is invalid";
const DETACHED_PEER_PID_RANGE_CONTEXT: &str = "detached Host peer PID exceeds process range";
const DETACHED_PEER_CAPTURE_CONTEXT: &str = "capture detached Host peer process identity";
const DETACHED_PEER_CREDENTIAL_CONTEXT: &str = "read detached Host peer credentials";
const INITIAL_STATUS_BIND_CONTEXT: &str = "bind initial authenticated detached Host status";
const CONFIRM_STATUS_BIND_CONTEXT: &str = "confirm authenticated detached Host status";
const STATUS_READ_CONTEXT: &str = "read authenticated detached Host status";
const CONNECT_PROBE_CONTEXT: &str = "connect detached Host identity probe";

pub(super) struct GuardArmFailure {
    error: anyhow::Error,
    guard: Option<DetachedRuntimeGuard>,
}

impl GuardArmFailure {
    pub(super) fn into_parts(self) -> (anyhow::Error, Option<DetachedRuntimeGuard>) {
        (self.error, self.guard)
    }

    pub(super) fn unbound(error: anyhow::Error) -> Self {
        Self { error, guard: None }
    }

    fn bound(error: anyhow::Error, guard: DetachedRuntimeGuard) -> Self {
        Self {
            error,
            guard: Some(guard),
        }
    }
}

pub(super) async fn arm_runtime_guard(
    expected_home: &Path,
) -> std::result::Result<DetachedRuntimeGuard, GuardArmFailure> {
    let deadline = Instant::now() + DETACHED_HOST_PUBLICATION_TIMEOUT;
    let mut guard = peer_guard_until(expected_home, deadline)
        .await
        .map_err(GuardArmFailure::unbound)?;
    let socket = match Paths::new(expected_home).host_socket() {
        Ok(socket) => socket,
        Err(error) => return Err(GuardArmFailure::bound(error, guard)),
    };
    let client = match Client::managed_local(expected_home, socket) {
        Ok(client) => client,
        Err(error) => return Err(GuardArmFailure::bound(anyhow!(error), guard)),
    };
    if let Err(error) = bind_statuses(&client, &mut guard, deadline).await {
        return Err(GuardArmFailure::bound(error, guard));
    }
    Ok(guard)
}

async fn peer_guard_until(expected_home: &Path, deadline: Instant) -> Result<DetachedRuntimeGuard> {
    let socket = Paths::new(expected_home).host_socket()?;
    let stream = connect_peer_until(&socket, deadline).await?;
    let credentials = stream
        .peer_cred()
        .context(DETACHED_PEER_CREDENTIAL_CONTEXT)?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    let expected_uid = unsafe { libc::geteuid() };
    ensure!(
        credentials.uid() == expected_uid,
        DETACHED_PEER_UID_CHANGED_DETAIL
    );
    let pid = credentials
        .pid()
        .context(DETACHED_PEER_PID_MISSING_DETAIL)?;
    ensure!(
        pid > libc::pid_t::default(),
        DETACHED_PEER_PID_INVALID_DETAIL
    );
    let pid = u32::try_from(pid).context(DETACHED_PEER_PID_RANGE_CONTEXT)?;
    let identity = ProcessIdentity::capture(pid).context(DETACHED_PEER_CAPTURE_CONTEXT)?;
    Ok(DetachedRuntimeGuard::from_peer(expected_home, identity))
}

async fn bind_statuses(
    client: &Client,
    guard: &mut DetachedRuntimeGuard,
    deadline: Instant,
) -> Result<()> {
    let first = read_status_until(client, deadline).await?;
    guard
        .verify_status(&first)
        .context(INITIAL_STATUS_BIND_CONTEXT)?;
    let second = read_status_until(client, deadline).await?;
    guard
        .verify_status(&second)
        .context(CONFIRM_STATUS_BIND_CONTEXT)
}

pub(super) async fn read_status_until(client: &Client, deadline: Instant) -> Result<HostStatus> {
    loop {
        match tokio::time::timeout_at(deadline, client.status()).await {
            Ok(Ok(status)) => return Ok(status),
            Ok(Err(error)) if retryable(&error) && Instant::now() < deadline => {
                sleep_for_retry(deadline).await;
            }
            Ok(Err(error)) => return Err(anyhow!(error)).context(STATUS_READ_CONTEXT),
            Err(error) => return Err(anyhow!(error)).context(STATUS_READ_CONTEXT),
        }
    }
}

pub(super) async fn connect_peer_until(path: &Path, deadline: Instant) -> Result<UnixStream> {
    loop {
        match tokio::time::timeout_at(deadline, UnixStream::connect(path)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(error)) if transient_io(error.kind()) && Instant::now() < deadline => {
                sleep_for_retry(deadline).await;
            }
            Ok(Err(error)) => return Err(error).context(CONNECT_PROBE_CONTEXT),
            Err(error) => return Err(anyhow!(error)).context(CONNECT_PROBE_CONTEXT),
        }
    }
}

async fn sleep_for_retry(deadline: Instant) {
    let wake = Instant::now() + DETACHED_HOST_PROBE_POLL_INTERVAL;
    tokio::time::sleep_until(std::cmp::min(wake, deadline)).await;
}

pub(super) fn retryable(error: &ClientError) -> bool {
    match error {
        ClientError::Protocol(ProtocolError::Io(source)) => transient_io(source.kind()),
        ClientError::Protocol(
            ProtocolError::EndOfStream
            | ProtocolError::TruncatedHeader { .. }
            | ProtocolError::OutcomeUnknown,
        ) => true,
        _ => false,
    }
}

fn transient_io(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
    )
}
