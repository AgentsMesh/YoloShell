use super::super::constants::SSH_OPERATION_TIMEOUT;
use super::super::*;
use super::support::{RuntimeBehavior, connection, known_hosts};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::Instant;

const REMOTE_HOME: &str = "/var/lib/yolo";
const REMOTE_RUNTIME: &str = "/opt/yolo/yolo";
const TEST_STAGE_DELAY: Duration = Duration::from_secs(20);

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::new(REMOTE_RUNTIME, REMOTE_HOME).unwrap()
}

#[tokio::test(start_paused = true)]
async fn hello_and_status_stalls_use_the_connection_operation_deadline() {
    let _clock = DrivenClock::new();
    assert_connect_timeout(RuntimeBehavior::StallHello).await;
    assert_connect_timeout(RuntimeBehavior::StallStatus).await;
}

#[tokio::test(start_paused = true)]
async fn attachment_stall_uses_the_channel_operation_deadline() {
    let _clock = DrivenClock::new();
    let connection = connection(REMOTE_HOME, "boot-a");
    let host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    let reached = Arc::new(Notify::new());
    connection.set_runtime_behaviors([RuntimeBehavior::StallAttachment(reached.clone())]);
    let mut operation = Box::pin(host.open_attachment(uuid::Uuid::new_v4(), 0));
    wait_until_reached(&mut operation, &reached).await;
    tokio::time::advance(SSH_OPERATION_TIMEOUT).await;
    assert_uncertain_timeout(operation.await.err().unwrap());
}

#[tokio::test(start_paused = true)]
async fn hello_and_status_share_one_absolute_deadline() {
    let _clock = DrivenClock::new();
    let connection = connection(REMOTE_HOME, "boot-a");
    let delay_started = Arc::new(Notify::new());
    let reached = Arc::new(Notify::new());
    connection.set_runtime_behaviors([RuntimeBehavior::DelayHelloThenStallStatus {
        delay: TEST_STAGE_DELAY,
        started: delay_started.clone(),
        reached: reached.clone(),
    }]);
    let task = spawn_connect(connection);
    delay_started.notified().await;
    let started = Instant::now();
    tokio::time::advance(TEST_STAGE_DELAY).await;
    reached.notified().await;
    tokio::time::advance(remaining_deadline()).await;
    assert_uncertain_timeout(task.await.unwrap());
    assert_eq!(Instant::now() - started, SSH_OPERATION_TIMEOUT);
}

#[tokio::test(start_paused = true)]
async fn channel_open_and_attach_share_one_absolute_deadline() {
    let _clock = DrivenClock::new();
    let connection = connection(REMOTE_HOME, "boot-a");
    let host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    let reached = Arc::new(Notify::new());
    connection.set_runtime_behaviors([RuntimeBehavior::DelayOpenThenStallAttachment {
        delay: TEST_STAGE_DELAY,
        reached: reached.clone(),
    }]);
    let started = Instant::now();
    let mut operation = Box::pin(host.open_attachment(uuid::Uuid::new_v4(), 0));
    poll_once(&mut operation).await;
    tokio::time::advance(TEST_STAGE_DELAY).await;
    wait_until_reached(&mut operation, &reached).await;
    tokio::time::advance(remaining_deadline()).await;
    assert_uncertain_timeout(operation.await.err().unwrap());
    assert_eq!(Instant::now() - started, SSH_OPERATION_TIMEOUT);
}

async fn assert_connect_timeout(behavior: impl FnOnce(Arc<Notify>) -> RuntimeBehavior) {
    let connection = connection(REMOTE_HOME, "boot-a");
    let reached = Arc::new(Notify::new());
    connection.set_runtime_behaviors([behavior(reached.clone())]);
    let task = spawn_connect(connection);
    reached.notified().await;
    tokio::time::advance(SSH_OPERATION_TIMEOUT).await;
    assert_uncertain_timeout(task.await.unwrap());
}

fn spawn_connect(
    connection: Arc<super::support::FakeConnection>,
) -> tokio::task::JoinHandle<RemoteError> {
    tokio::spawn(async move {
        RemoteHost::connect(connection, &known_hosts(), runtime())
            .await
            .err()
            .unwrap()
    })
}

fn remaining_deadline() -> Duration {
    SSH_OPERATION_TIMEOUT.checked_sub(TEST_STAGE_DELAY).unwrap()
}

fn assert_uncertain_timeout(error: RemoteError) {
    assert!(matches!(
        error,
        RemoteError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
    ));
}

async fn wait_until_reached<T>(
    operation: &mut std::pin::Pin<Box<impl Future<Output = Result<T, RemoteError>>>>,
    reached: &Notify,
) {
    tokio::select! {
        result = operation => {
            result.err().unwrap();
            unreachable!();
        }
        () = reached.notified() => {}
    }
}

async fn poll_once<T>(
    operation: &mut std::pin::Pin<Box<impl Future<Output = Result<T, RemoteError>>>>,
) {
    tokio::select! {
        biased;
        result = operation => {
            result.err().unwrap();
            unreachable!();
        }
        () = tokio::task::yield_now() => {}
    }
}

// The RPC transport runs on an independent LocalSet. Keep this paused domain
// executor runnable so it only advances time at the explicit assertions below.
struct DrivenClock(tokio::task::JoinHandle<()>);
impl DrivenClock {
    fn new() -> Self {
        Self(tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        }))
    }
}
impl Drop for DrivenClock {
    fn drop(&mut self) {
        self.0.abort();
    }
}
