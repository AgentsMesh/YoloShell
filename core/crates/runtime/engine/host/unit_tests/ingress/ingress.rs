use crate::host::tests::*;
use super::*;

pub(super) fn spawn_admitted_host_client(
    host: Arc<Host>,
    clients: &Arc<tokio::sync::Semaphore>,
) -> (UnixStream, tokio::task::JoinHandle<Result<()>>) {
    let (client, server) = UnixStream::pair().unwrap();
    let permit = crate::ingress::try_admit(clients).unwrap();
    let task = tokio::spawn(async move {
        let _permit = permit;
        host.handle_client(server).await
    });
    (client, task)
}

#[test]
fn host_admission_is_bounded_at_the_configured_limit() {
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let permits: Vec<_> = (0..MAX_HOST_CLIENTS)
        .map(|_| crate::ingress::try_admit(&clients).unwrap())
        .collect();
    assert!(crate::ingress::try_admit(&clients).is_none());
    drop(permits);
    assert!(crate::ingress::try_admit(&clients).is_some());
}

#[tokio::test(start_paused = true)]
async fn silent_first_request_times_out_and_releases_its_slot() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(1));
    let (_client, task) = spawn_admitted_host_client(host, &clients);
    tokio::task::yield_now().await;
    assert!(crate::ingress::try_admit(&clients).is_none());

    tokio::time::advance(HOST_REQUEST_TIMEOUT).await;
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(HOST_REQUEST_TIMEOUT_CONTEXT));
    assert!(crate::ingress::try_admit(&clients).is_some());
}

#[tokio::test(start_paused = true)]
async fn idle_client_after_status_times_out_and_releases_its_slot() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let clients = Arc::new(tokio::sync::Semaphore::new(1));
    let (client, task) = spawn_admitted_host_client(host, &clients);
    let client = open_host_rpc(client).await;
    let response = host_rpc(&client, &request(method::HOST_STATUS, &())).await;
    assert!(response.ok);
    assert!(crate::ingress::try_admit(&clients).is_none());

    tokio::time::advance(HOST_REQUEST_TIMEOUT).await;
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(HOST_REQUEST_TIMEOUT_CONTEXT));
    assert!(crate::ingress::try_admit(&clients).is_some());
}
