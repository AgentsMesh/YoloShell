use crate::worker::tests::*;
use tokio::io::AsyncReadExt;

const SATURATED_ATTACH_REQUEST_ID: &str = "saturated-attach";
const RESERVED_STATUS_REQUEST_ID: &str = "reserved-control-status";

#[test]
fn worker_admission_rejects_max_plus_one_without_waiting() {
    let clients = Arc::new(Semaphore::new(MAX_WORKER_CLIENTS));
    let permits = (0..MAX_WORKER_CLIENTS)
        .map(|_| crate::ingress::try_admit(&clients).unwrap())
        .collect::<Vec<_>>();
    assert!(crate::ingress::try_admit(&clients).is_none());
    drop(permits);
    assert!(crate::ingress::try_admit(&clients).is_some());
}

#[test]
fn saturated_worker_attachments_leave_the_control_reserve_available() {
    let clients = Arc::new(Semaphore::new(MAX_WORKER_CLIENTS));
    let attachments = Arc::new(Semaphore::new(MAX_WORKER_ATTACHMENTS));
    let client_permits = (0..MAX_WORKER_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&clients).unwrap())
        .collect::<Vec<_>>();
    let attachment_permits = (0..MAX_WORKER_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&attachments).unwrap())
        .collect::<Vec<_>>();
    assert!(crate::ingress::try_admit(&attachments).is_none());
    assert_eq!(clients.available_permits(), WORKER_CONTROL_CLIENT_RESERVE);
    drop((client_permits, attachment_permits));
}

#[tokio::test]
async fn attachment_saturation_does_not_block_worker_status() {
    let fixture = Fixture::new();
    let permits = (0..MAX_WORKER_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&fixture.state.attachment_clients).unwrap())
        .collect::<Vec<_>>();
    let (mut client, task) = authenticated_worker_connection(&fixture, 1).await;
    let attach = RequestEnvelope::new(
        SATURATED_ATTACH_REQUEST_ID,
        crate::protocol::method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: 0,
        })
        .unwrap(),
    );
    let rejected = client.exchange(&attach).await.unwrap();
    assert_eq!(rejected.error.unwrap().message, WORKER_ATTACHMENT_LIMIT_MESSAGE);

    let status = RequestEnvelope::new(
        RESERVED_STATUS_REQUEST_ID,
        WORKER_STATUS_METHOD,
        to_value(&TerminalRef {
            terminal_id: fixture.terminal_id(),
        })
        .unwrap(),
    );
    let response = client.exchange(&status).await.unwrap();
    assert!(response.ok);
    drop((client, permits));
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn saturated_worker_connection_is_closed_without_a_task() {
    let fixture = Fixture::new();
    let clients = Arc::new(Semaphore::new(1));
    let _permit = crate::ingress::try_admit(&clients).unwrap();
    let (server, mut client) = UnixStream::pair().unwrap();
    let address = server.peer_addr().unwrap();
    let mut tasks = JoinSet::new();
    let mut failures = 0;

    let retry = handle_accept_result(
        Ok((server, address)),
        &mut tasks,
        &fixture.state,
        &clients,
        &mut failures,
    );

    assert!(retry.is_none());
    let mut byte = [0_u8; 1];
    assert_eq!(client.read(&mut byte).await.unwrap(), 0);
    assert!(tasks.is_empty());
}

#[tokio::test(start_paused = true)]
async fn stalled_worker_authentication_releases_its_slot() {
    let fixture = Fixture::new();
    let clients = Arc::new(Semaphore::new(1));
    let (server, _client) = UnixStream::pair().unwrap();
    let address = server.peer_addr().unwrap();
    let mut tasks = JoinSet::new();
    let mut failures = 0;

    let retry = handle_accept_result(
        Ok((server, address)),
        &mut tasks,
        &fixture.state,
        &clients,
        &mut failures,
    );
    assert!(retry.is_none());
    assert_eq!(clients.available_permits(), 0);
    tokio::time::advance(WORKER_AUTH_TIMEOUT).await;
    let _ = tasks.join_next().await;
    assert_eq!(clients.available_permits(), 1);
}
