use crate::host::tests::*;
use super::*;

const INGRESS_PANIC_SOCKET: &str = "ingress-panic.sock";
const INGRESS_LOOP_TEST_PANIC: &str = "injected ingress event-loop panic";
const INGRESS_CLIENT_OWNER_NOT_STARTED: &str = "ingress client owner did not start";
const INGRESS_IDLE_OWNER_NOT_STARTED: &str = "ingress idle owner did not start";
const INGRESS_CLIENT_OWNER_NOT_DROPPED: &str = "ingress client owner was not dropped";
const INGRESS_IDLE_OWNER_NOT_DROPPED: &str = "ingress idle owner was not dropped";
const INGRESS_PANIC_HOST_OWNER_SURVIVED: &str = "ingress panic retained Host ownership";
const INGRESS_RUNTIME_NOT_FINISHED: &str = "ingress runtime did not finish";
const INGRESS_DRAIN_SUBSCRIBER_PANIC: &str = "injected ingress drain subscriber panic";
const INGRESS_DRAIN_WARNING_NOT_OBSERVED: &str = "ingress drain warning was not observed";
const INGRESS_DRAIN_OWNER_NOT_DROPPED: &str = "ingress drain client owner was not dropped";
const INGRESS_DRAIN_OWNER_SURVIVED: &str = "ingress drain retained Host ownership";
const INGRESS_TEST_SPAN_ID: u64 = 1;

struct IngressOwnerDropProbe {
    dropped: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Drop for IngressOwnerDropProbe {
    fn drop(&mut self) {
        if let Some(dropped) = self.dropped.take() {
            let _ = dropped.send(());
        }
    }
}

struct IngressDrainPanicSubscriber(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl tracing::Subscriber for IngressDrainPanicSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(INGRESS_TEST_SPAN_ID)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
        panic!("{INGRESS_DRAIN_SUBSCRIBER_PANIC}");
    }

    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn ingress_loop_panic_drains_client_and_idle_owners() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let listener = UnixListener::bind(directory.path().join(INGRESS_PANIC_SOCKET)).unwrap();
    let mut tasks = HostIngressTasks::new();
    let (client_started, client_ready) = tokio::sync::oneshot::channel();
    let (client_dropped, client_drop) = tokio::sync::oneshot::channel();
    let client_owner = Arc::clone(&host);
    tasks.clients.spawn(async move {
        let _owner = client_owner;
        let _probe = IngressOwnerDropProbe {
            dropped: Some(client_dropped),
        };
        let _ = client_started.send(());
        std::future::pending::<()>().await;
    });
    let (idle_started, idle_ready) = tokio::sync::oneshot::channel();
    let (idle_dropped, idle_drop) = tokio::sync::oneshot::channel();
    let idle_owner = Arc::clone(&host);
    tasks.idle_shutdown.spawn(async move {
        let _owner = idle_owner;
        let _probe = IngressOwnerDropProbe {
            dropped: Some(idle_dropped),
        };
        let _ = idle_started.send(());
        std::future::pending::<()>().await;
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, client_ready)
        .await
        .expect(INGRESS_CLIENT_OWNER_NOT_STARTED)
        .expect(INGRESS_CLIENT_OWNER_NOT_STARTED);
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, idle_ready)
        .await
        .expect(INGRESS_IDLE_OWNER_NOT_STARTED)
        .expect(INGRESS_IDLE_OWNER_NOT_STARTED);
    tasks.fault = Some(Box::pin(async { panic!("{INGRESS_LOOP_TEST_PANIC}") }));
    let (armed, ready) = tokio::sync::oneshot::channel();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();

    let error = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        run_host_ingress_runtime(Arc::clone(&host), listener, armed, receiver, tasks),
    )
    .await
    .expect(INGRESS_RUNTIME_NOT_FINISHED)
    .unwrap_err();

    ready.await.unwrap();
    assert!(error.to_string().contains(HOST_INGRESS_LOOP_PANIC_CONTEXT));
    assert!(format!("{error:#}").contains(INGRESS_LOOP_TEST_PANIC));
    assert!(host.request_drain.admit_response().is_err());
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, client_drop)
        .await
        .expect(INGRESS_CLIENT_OWNER_NOT_DROPPED)
        .expect(INGRESS_CLIENT_OWNER_NOT_DROPPED);
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, idle_drop)
        .await
        .expect(INGRESS_IDLE_OWNER_NOT_DROPPED)
        .expect(INGRESS_IDLE_OWNER_NOT_DROPPED);
    drop(shutdown);
    drop(
        Arc::try_unwrap(host)
            .unwrap_or_else(|_| panic!("{INGRESS_PANIC_HOST_OWNER_SURVIVED}")),
    );
    Host::open(HostConfig::new(directory.path())).unwrap();
}

#[tokio::test(start_paused = true)]
async fn response_drain_subscriber_panic_still_joins_client_owner() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let warning_observed = Arc::new(std::sync::atomic::AtomicBool::default());
    let subscriber = tracing::subscriber::set_default(IngressDrainPanicSubscriber(Arc::clone(
        &warning_observed,
    )));
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let _response = host.request_drain.admit_response().unwrap();
    let mut client_tasks = tokio::task::JoinSet::new();
    let mut idle_tasks = tokio::task::JoinSet::new();
    let owner = Arc::clone(&host);
    let (started, ready) = tokio::sync::oneshot::channel();
    let (dropped, owner_drop) = tokio::sync::oneshot::channel();
    client_tasks.spawn(async move {
        let _owner = owner;
        let _probe = IngressOwnerDropProbe {
            dropped: Some(dropped),
        };
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, ready)
        .await
        .expect(INGRESS_CLIENT_OWNER_NOT_STARTED)
        .expect(INGRESS_CLIENT_OWNER_NOT_STARTED);

    tokio::time::timeout(
        HOST_RESPONSE_DRAIN_TIMEOUT + HOST_BLOCKING_TEST_TIMEOUT,
        shutdown_ingress_tasks(&host, &mut client_tasks, &mut idle_tasks),
    )
    .await
    .expect(INGRESS_RUNTIME_NOT_FINISHED);

    assert!(
        warning_observed.load(std::sync::atomic::Ordering::Acquire),
        "{INGRESS_DRAIN_WARNING_NOT_OBSERVED}"
    );
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, owner_drop)
        .await
        .expect(INGRESS_DRAIN_OWNER_NOT_DROPPED)
        .expect(INGRESS_DRAIN_OWNER_NOT_DROPPED);
    drop(subscriber);
    drop(_response);
    drop(
        Arc::try_unwrap(host).unwrap_or_else(|_| panic!("{INGRESS_DRAIN_OWNER_SURVIVED}")),
    );
    Host::open(HostConfig::new(directory.path())).unwrap();
}
