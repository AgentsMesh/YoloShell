use crate::host::tests::*;
use super::*;

const ATTACHMENT_AUTH_IDENTITY_SOCKET: &str = "attachment-auth-identity.sock";
const ATTACHMENT_AUTH_AFTER_SEQUENCE: u64 = 0;

#[tokio::test]
async fn attachment_auth_identity_mismatch_is_not_retryable() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ATTACHMENT_AUTH_IDENTITY_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let mut reported = record.clone();
    reported.worker_id = Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let response = ResponseEnvelope::success(&auth, &test_worker_hello(&reported)).unwrap();
        write_frame(&mut stream, &response).await.unwrap();
    });
    let error = host
        .rpc_attachment(&request(
            method::ATTACH_SESSION,
            &AttachRequest {
                terminal_id,
                after_seq: ATTACHMENT_AUTH_AFTER_SEQUENCE,
            },
        ))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!error.retryable);
    server.await.unwrap();
}
