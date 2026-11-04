use super::*;
use std::future::{pending, ready};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant as WallInstant};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const SIGNAL_CLOSED: &str = "bridge deadline signal closed";
const STALLED_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_HOST_SOCKET_FILE: &str = "host.sock";
const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_ARGUMENT: &str = "60";
const TEST_WALL_TIMEOUT: Duration = Duration::from_secs(2);

struct AbortTask<T>(Option<JoinHandle<T>>);

impl<T> AbortTask<T> {
    fn new(task: JoinHandle<T>) -> Self {
        Self(Some(task))
    }

    async fn join(mut self) -> T {
        let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
        while !self.0.as_ref().unwrap().is_finished() {
            assert!(WallInstant::now() < deadline);
            tokio::task::yield_now().await;
        }
        self.0.take().unwrap().await.unwrap()
    }

    async fn abort(mut self) {
        let task = self.0.take().unwrap();
        task.abort();
        let _ = task.await;
    }
}

impl<T> Drop for AbortTask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

async fn receive<T>(receiver: &mut oneshot::Receiver<T>) -> T {
    let deadline = WallInstant::now() + TEST_WALL_TIMEOUT;
    loop {
        match receiver.try_recv() {
            Ok(value) => return value,
            Err(oneshot::error::TryRecvError::Closed) => panic!("{SIGNAL_CLOSED}"),
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        assert!(WallInstant::now() < deadline);
        tokio::task::yield_now().await;
    }
}

fn assert_host_not_ready(error: anyhow::Error, socket: &Path) {
    assert_eq!(
        error.to_string(),
        format!("{HOST_NOT_READY_CONTEXT}: {}", socket.display())
    );
}

#[tokio::test(start_paused = true)]
async fn exhausted_initial_probe_does_not_spawn_and_cleans_captured_child() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let (accepted, mut observed) = oneshot::channel();
    let server = AbortTask::new(tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        accepted.send(()).unwrap();
        let _stream = stream;
        pending::<()>().await;
    }));
    let child = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let child_pid = child.id();
    let spawned = SpawnedHost::new(child);
    let spawn_observed = Arc::new(AtomicBool::new(false));
    let spawn_in_task = Arc::clone(&spawn_observed);
    let paths = Paths::new(home.path());
    let task_socket = socket.clone();
    let started = tokio::time::Instant::now();
    let deadline = started + STALLED_PROBE_TIMEOUT;
    let operation = AbortTask::new(tokio::spawn(async move {
        ensure_host_running_until(
            &paths,
            &task_socket,
            deadline,
            move |_, _| {
                spawn_in_task.store(true, Ordering::Release);
                Ok(spawned)
            },
            ready(()),
        )
        .await
    }));

    receive(&mut observed).await;
    tokio::time::advance(deadline.saturating_duration_since(tokio::time::Instant::now())).await;
    assert_host_not_ready(operation.join().await.unwrap_err(), &socket);
    assert_eq!(tokio::time::Instant::now() - started, STALLED_PROBE_TIMEOUT);
    assert!(!spawn_observed.load(Ordering::Acquire));
    assert!(!yolo_runtime::process::is_process_alive(child_pid));
    server.abort().await;
}

#[tokio::test(start_paused = true)]
async fn expired_budget_after_argument_construction_still_prevents_spawn() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = AbortTask::new(tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    }));
    let child = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let child_pid = child.id();
    let spawned = SpawnedHost::new(child);
    let spawn_observed = Arc::new(AtomicBool::new(false));
    let spawn_in_task = Arc::clone(&spawn_observed);
    let (barrier_entered, mut barrier_observed) = oneshot::channel();
    let (release_barrier, barrier_release) = oneshot::channel();
    let paths = Paths::new(home.path());
    let task_socket = socket.clone();
    let deadline = tokio::time::Instant::now() + STALLED_PROBE_TIMEOUT;
    let operation = AbortTask::new(tokio::spawn(async move {
        ensure_host_running_until(
            &paths,
            &task_socket,
            deadline,
            move |_, _| {
                spawn_in_task.store(true, Ordering::Release);
                Ok(spawned)
            },
            async move {
                barrier_entered.send(()).unwrap();
                barrier_release.await.unwrap();
            },
        )
        .await
    }));

    receive(&mut barrier_observed).await;
    tokio::time::advance(deadline.saturating_duration_since(tokio::time::Instant::now())).await;
    release_barrier.send(()).unwrap();
    assert_host_not_ready(operation.join().await.unwrap_err(), &socket);
    assert!(!spawn_observed.load(Ordering::Acquire));
    assert!(!yolo_runtime::process::is_process_alive(child_pid));
    server.join().await;
}
