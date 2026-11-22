use crate::host::tests::*;
use super::*;

const IDLE_INGRESS_SOCKET: &str = "idle-ingress.sock";
const IDLE_INGRESS_SHUTDOWN_NOT_COMMITTED: &str =
    "last client disconnect did not commit idle Host shutdown";
const IDLE_TASK_DID_NOT_START: &str = "pending idle shutdown task did not start";
const IDLE_TASK_DROP_DID_NOT_START: &str = "pending idle shutdown task drop did not start";
const IDLE_TASK_DROP_NOT_RELEASED: &str = "pending idle shutdown task drop was not released";
const IDLE_TASK_OWNER_SURVIVED: &str = "pending idle shutdown task retained Host ownership";
const IDLE_TASK_SHUTDOWN_NOT_FINISHED: &str = "pending idle shutdown task did not finish";

struct IdleShutdownTaskDropBarrier {
    started: Option<tokio::sync::oneshot::Sender<()>>,
    release: std::sync::mpsc::Receiver<()>,
}

impl Drop for IdleShutdownTaskDropBarrier {
    fn drop(&mut self) {
        if let Some(started) = self.started.take() {
            let _ = started.send(());
        }
        self.release
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .expect(IDLE_TASK_DROP_NOT_RELEASED);
    }
}

#[tokio::test(start_paused = true)]
async fn last_client_disconnect_schedules_quiescent_host_shutdown() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let drain = host.request_drain.clone();
    let (mut ingress, ready) = idle_ingress(&directory, host);
    ready.await.unwrap();

    let client = idle_status_client(&directory).await;
    drop(client);
    settle_idle_shutdown(&drain).await;

    assert!(drain.is_draining());
    ingress.shutdown().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn new_client_cancels_pending_automatic_shutdown_until_it_disconnects() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let drain = host.request_drain.clone();
    let (mut ingress, ready) = idle_ingress(&directory, host);
    ready.await.unwrap();

    drop(idle_status_client(&directory).await);
    tokio::task::yield_now().await;
    let replacement = idle_status_client(&directory).await;
    tokio::time::advance(HOST_IDLE_SHUTDOWN_QUIET_PERIOD).await;
    tokio::task::yield_now().await;
    assert!(!drain.is_draining());

    drop(replacement);
    settle_idle_shutdown(&drain).await;
    assert!(drain.is_draining());
    ingress.shutdown().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn abort_during_quiet_period_never_closes_request_admission() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let client_tasks = tokio::task::JoinSet::new();
    let mut shutdown_tasks = tokio::task::JoinSet::new();
    schedule_idle_shutdown(&host, &clients, &client_tasks, &mut shutdown_tasks);

    shutdown_tasks.abort_all();
    while shutdown_tasks.join_next().await.is_some() {}
    tokio::time::advance(HOST_IDLE_SHUTDOWN_QUIET_PERIOD).await;

    assert!(!host.request_drain.is_draining());
    assert!(host.request_drain.admit_mutation().await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ingress_shutdown_closes_admission_before_joining_idle_task_owners() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let mut client_tasks = tokio::task::JoinSet::new();
    let mut idle_tasks = tokio::task::JoinSet::new();
    let owner = Arc::clone(&host);
    let (started, ready) = tokio::sync::oneshot::channel();
    let (drop_started, dropping) = tokio::sync::oneshot::channel();
    let (release_drop, drop_release) = std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let drop_barrier = IdleShutdownTaskDropBarrier {
        started: Some(drop_started),
        release: drop_release,
    };
    idle_tasks.spawn(async move {
        let _owner = owner;
        let _drop_barrier = drop_barrier;
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, ready)
        .await
        .expect(IDLE_TASK_DID_NOT_START)
        .expect(IDLE_TASK_DID_NOT_START);
    let shutting_host = Arc::clone(&host);
    let shutdown = tokio::spawn(async move {
        shutdown_ingress_tasks(&shutting_host, &mut client_tasks, &mut idle_tasks).await;
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, dropping)
        .await
        .expect(IDLE_TASK_DROP_DID_NOT_START)
        .expect(IDLE_TASK_DROP_DID_NOT_START);

    assert!(host.request_drain.admit_response().is_err());
    release_drop.send(()).unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, shutdown)
        .await
        .expect(IDLE_TASK_SHUTDOWN_NOT_FINISHED)
        .unwrap();

    drop(Arc::try_unwrap(host).unwrap_or_else(|_| panic!("{IDLE_TASK_OWNER_SURVIVED}")));
    Host::open(HostConfig::new(directory.path())).unwrap();
}

async fn settle_idle_shutdown(drain: &request_drain::HostRequestDrain) {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    let committed = drain.wait_for_shutdown();
    tokio::pin!(committed);
    loop {
        tokio::select! {
            biased;
            outcome = &mut committed => {
                outcome.expect(IDLE_INGRESS_SHUTDOWN_NOT_COMMITTED);
                return;
            }
            _ = tokio::task::yield_now() => {}
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{IDLE_INGRESS_SHUTDOWN_NOT_COMMITTED}"
        );
        // EOF is delivered by the independent RPC executor. Advance only the quiet
        // period; once preparation starts, let its real completion commit shutdown.
        if !drain.is_draining() {
            tokio::time::advance(HOST_IDLE_SHUTDOWN_QUIET_PERIOD).await;
        }
    }
}

fn idle_ingress(
    directory: &tempfile::TempDir,
    host: Arc<Host>,
) -> (HostIngress, tokio::sync::oneshot::Receiver<IngressArmed>) {
    let listener = UnixListener::bind(directory.path().join(IDLE_INGRESS_SOCKET)).unwrap();
    let (armed, ready) = tokio::sync::oneshot::channel();
    (HostIngress::spawn(host, listener, armed), ready)
}

async fn idle_status_client(directory: &tempfile::TempDir) -> yolo_capnp_rpc::Connection {
    let client = UnixStream::connect(directory.path().join(IDLE_INGRESS_SOCKET))
        .await
        .unwrap();
    let client = open_host_rpc(client).await;
    let _: ResponseEnvelope = host_rpc(&client, &request(method::HOST_STATUS, &())).await;
    client
}
