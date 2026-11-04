use crate::host::tests::*;
use super::*;

#[test]
fn saturated_host_attachments_leave_the_control_reserve_available() {
    let clients = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));
    let attachments = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_ATTACHMENTS));
    let client_permits = (0..MAX_HOST_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&clients).unwrap())
        .collect::<Vec<_>>();
    let attachment_permits = (0..MAX_HOST_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&attachments).unwrap())
        .collect::<Vec<_>>();
    assert!(crate::ingress::try_admit(&attachments).is_none());
    assert_eq!(clients.available_permits(), HOST_CONTROL_CLIENT_RESERVE);
    assert!(crate::ingress::try_admit(&clients).is_some());
    drop((client_permits, attachment_permits));
}

#[tokio::test]
async fn attachment_saturation_does_not_block_host_status_on_the_same_client() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let permits = (0..MAX_HOST_ATTACHMENTS)
        .map(|_| crate::ingress::try_admit(&host.attachment_clients).unwrap())
        .collect::<Vec<_>>();
    let (mut client, server) = UnixStream::pair().unwrap();
    let task = tokio::spawn(Arc::clone(&host).handle_client(server));
    let client = open_host_rpc(client).await;
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id: Uuid::new_v4(),
            after_seq: 0,
        },
    );
    let rejected = host_rpc(&client, &attach).await;
    assert_eq!(
        rejected.error.unwrap().message,
        HOST_ATTACHMENT_LIMIT_MESSAGE
    );

    let status = host_rpc(&client, &request(method::HOST_STATUS, &())).await;
    assert!(status.ok);
    drop((client, permits));
    assert!(task.await.unwrap().is_ok());
}
