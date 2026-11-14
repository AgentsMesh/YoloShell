use super::*;
use std::future::pending;
use std::path::PathBuf;
use std::time::{Duration, Instant as WallInstant};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const HOST_SOCKET_NAME: &str = "host-ready-deadline.sock";
const SIGNAL_CLOSED: &str = "host-ready probe signal closed";
const TEST_HOST_EXECUTABLE: &str = crate::test_executables::TRUE;
const TEST_WALL_TIMEOUT: Duration = Duration::from_secs(2);

async fn receive(receiver: &mut oneshot::Receiver<()>) {
    let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
    loop {
        match receiver.try_recv() {
            Ok(()) => return,
            Err(oneshot::error::TryRecvError::Closed) => panic!("{SIGNAL_CLOSED}"),
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
}

async fn join<T>(task: JoinHandle<T>) -> T {
    let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
    while !task.is_finished() {
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
    task.await.unwrap()
}

#[tokio::test(start_paused = true)]
async fn post_spawn_status_probe_cannot_exceed_host_ready_deadline() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(HOST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (stalled_probe, mut probe_observed) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (initial, _) = listener.accept().await.unwrap();
        drop(initial);
        let (stalled, _) = listener.accept().await.unwrap();
        stalled_probe.send(()).unwrap();
        let _stalled = stalled;
        pending::<()>().await;
    });
    let runtime = RelayRuntime::managed(
        home.path(),
        &socket,
        Some(PathBuf::from(TEST_HOST_EXECUTABLE)),
    )
    .unwrap();
    let started = tokio::time::Instant::now();
    let operation = tokio::spawn(async move { runtime.ensure_host().await });
    receive(&mut probe_observed).await;

    tokio::time::advance(HOST_READY_TIMEOUT).await;
    let error = join(operation).await.unwrap_err();
    assert!(matches!(
        error,
        RelayError::HostStartupTimeout { socket: actual } if actual == socket
    ));
    assert_eq!(tokio::time::Instant::now() - started, HOST_READY_TIMEOUT);
    server.abort();
    let _ = server.await;
}
