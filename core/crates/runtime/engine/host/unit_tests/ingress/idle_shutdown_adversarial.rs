use crate::host::tests::*;
use super::*;

const IDLE_DRAIN_YIELD_ATTEMPTS: usize = 32;
const IDLE_ADVERSARIAL_SOCKET: &str = "idle-adversarial.sock";
const AUTHENTICATED_SHUTDOWN_TASK_JOIN_FAILED: &str = "authenticated shutdown scenario task failed";

#[tokio::test]
async fn accepted_socket_without_a_permit_cannot_cross_quiescent_commit() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let listener = UnixListener::bind(directory.path().join(IDLE_ADVERSARIAL_SOCKET)).unwrap();
    let client = UnixStream::connect(directory.path().join(IDLE_ADVERSARIAL_SOCKET))
        .await
        .unwrap();
    let accepted = listener
        .accept()
        .await
        .map(|(stream, address)| (UnixStream::from(stream), address));
    let preparation = host
        .prepare_quiescent_shutdown(Arc::clone(&clients))
        .await
        .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    let mut backoff =
        crate::ingress::AcceptBackoff::new(HOST_ACCEPT_INITIAL_BACKOFF, HOST_ACCEPT_MAX_BACKOFF);

    handle_host_accept(&host, &clients, &mut tasks, &mut backoff, accepted).await;

    assert!(tasks.is_empty());
    preparation.commit();
    drop(client);
}

#[tokio::test]
async fn commit_before_client_permit_still_rejects_the_accepted_mutation() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let listener = UnixListener::bind(directory.path().join(IDLE_ADVERSARIAL_SOCKET)).unwrap();
    let mut client = UnixStream::connect(directory.path().join(IDLE_ADVERSARIAL_SOCKET))
        .await
        .unwrap();
    let accepted = listener
        .accept()
        .await
        .map(|(stream, address)| (UnixStream::from(stream), address));
    let preparation = host
        .prepare_quiescent_shutdown(Arc::clone(&clients))
        .await
        .unwrap();
    preparation.commit();
    let mut tasks = tokio::task::JoinSet::new();
    let mut backoff =
        crate::ingress::AcceptBackoff::new(HOST_ACCEPT_INITIAL_BACKOFF, HOST_ACCEPT_MAX_BACKOFF);

    handle_host_accept(&host, &clients, &mut tasks, &mut backoff, accepted).await;
    let mut client = open_host_rpc(client).await;
    let response = exchange_host_request(&mut client, method::CREATE_SESSION).await;

    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::CONFLICT
    );
    drop(client);
    assert!(tasks.join_next().await.unwrap().is_ok());
}

#[tokio::test]
async fn explicit_drain_allows_status_but_rejects_mutation_and_attachment() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let reconcile = Arc::clone(&host.reconcile_gate).lock_owned().await;
    let preparing_host = Arc::clone(&host);
    let preparing = tokio::spawn(async move { preparing_host.prepare_idle_shutdown().await });
    wait_until_draining(&host).await;
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let (mut client, task) = spawn_admitted_host_client(Arc::clone(&host), &clients);
    let mut client = open_host_rpc(client).await;

    let status = exchange_host_request(&mut client, method::HOST_STATUS).await;
    assert!(status.ok);
    let mutation = exchange_host_request(&mut client, method::CREATE_SESSION).await;
    assert_eq!(
        mutation.error.unwrap().code,
        crate::protocol::error_code::CONFLICT
    );
    let attachment = exchange_host_request(&mut client, method::ATTACH_SESSION).await;
    assert_eq!(
        attachment.error.unwrap().code,
        crate::protocol::error_code::CONFLICT
    );

    drop(client);
    drop(reconcile);
    drop(preparing.await.unwrap().unwrap());
    let _ = task.await;
    assert!(!host.request_drain.is_draining());
}

#[tokio::test]
async fn abort_during_catalog_verification_reopens_request_admission() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let catalog = Arc::clone(&host.catalog);
    let (locked, entered) = std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (release, released) = std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let holder = std::thread::spawn(move || {
        let _guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv().unwrap();
    });
    entered.recv().unwrap();
    let (operation, mut observed) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operation);
    let preparing_host = Arc::clone(&host);
    let preparing = tokio::spawn(async move { preparing_host.prepare_idle_shutdown().await });
    observed.recv().await.unwrap();

    preparing.abort();
    let joined = preparing.await;
    assert!(matches!(joined, Err(error) if error.is_cancelled()));
    assert!(!host.request_drain.is_draining());
    release.send(()).unwrap();
    holder.join().unwrap();
    assert!(host.request_drain.admit_mutation().await.is_ok());
}

#[tokio::test]
async fn abort_while_waiting_for_reconcile_reopens_request_admission() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let reconcile = Arc::clone(&host.reconcile_gate).lock_owned().await;
    let preparing_host = Arc::clone(&host);
    let preparing = tokio::spawn(async move { preparing_host.prepare_idle_shutdown().await });
    wait_until_draining(&host).await;

    preparing.abort();
    let joined = preparing.await;
    assert!(matches!(joined, Err(error) if error.is_cancelled()));
    assert!(!host.request_drain.is_draining());
    drop(reconcile);
    assert!(host.request_drain.admit_mutation().await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authenticated_shutdown_response_arrives_before_host_exit() {
    tokio::spawn(async move {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let socket = host.socket_path().to_path_buf();
        let client = yolo_client::Client::managed_local(directory.path(), &socket).unwrap();
        let serving = tokio::spawn(host.serve_until(std::future::pending(), |_| Ok(())));
        wait_for_host_socket(&socket).await;

        client.shutdown_if_idle().await.unwrap();

        tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, serving)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!socket.exists());
    })
    .await
    .expect(AUTHENTICATED_SHUTDOWN_TASK_JOIN_FAILED);
}

async fn exchange_host_request(
    client: &yolo_capnp_rpc::Connection,
    method_name: &str,
) -> ResponseEnvelope {
    host_rpc(client, &request(method_name, &())).await
}

async fn wait_until_draining(host: &Host) {
    for _ in 0..IDLE_DRAIN_YIELD_ATTEMPTS {
        if host.request_drain.is_draining() {
            return;
        }
        tokio::task::yield_now().await;
    }
    assert!(host.request_drain.is_draining());
}
