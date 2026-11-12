use crate::host::tests::*;
use super::*;

const ATTACHMENT_CORRELATION_SOCKET: &str = "attachment-correlation.sock";
const ATTACHMENT_CORRELATION_AFTER_SEQUENCE: u64 = 0;
const CORRUPTED_PROTOCOL_VERSION_OFFSET: u16 = 1;

#[derive(Clone, Copy)]
enum AttachmentCorrelationCorruption {
    Version,
    Kind,
    RequestId,
    Method,
}

#[tokio::test]
async fn attachment_response_correlation_errors_are_not_retryable() {
    for corruption in [
        AttachmentCorrelationCorruption::Version,
        AttachmentCorrelationCorruption::Kind,
        AttachmentCorrelationCorruption::RequestId,
        AttachmentCorrelationCorruption::Method,
    ] {
        assert_attachment_correlation_rejected(corruption).await;
    }
}

async fn assert_attachment_correlation_rejected(corruption: AttachmentCorrelationCorruption) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ATTACHMENT_CORRELATION_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let mut response =
            ResponseEnvelope::success(&request, &empty_attachment(record.runtime_epoch)).unwrap();
        corrupt_attachment_response(&mut response, corruption);
        assert!(matches!(
            stream.respond(&response).await,
            Err(ProtocolError::EndOfStream)
        ));
    });
    let error = host
        .rpc_attachment(&request(
            method::ATTACH_SESSION,
            &AttachRequest {
                terminal_id,
                after_seq: ATTACHMENT_CORRELATION_AFTER_SEQUENCE,
            },
        ))
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert!(!error.retryable);
    server.await.unwrap();
}

fn corrupt_attachment_response(
    response: &mut ResponseEnvelope,
    corruption: AttachmentCorrelationCorruption,
) {
    match corruption {
        AttachmentCorrelationCorruption::Version => {
            response.version = response
                .version
                .saturating_add(CORRUPTED_PROTOCOL_VERSION_OFFSET);
        }
        AttachmentCorrelationCorruption::Kind => {
            response.kind = crate::protocol::REQUEST_KIND.to_owned();
        }
        AttachmentCorrelationCorruption::RequestId => {
            response.request_id = Uuid::new_v4().to_string();
        }
        AttachmentCorrelationCorruption::Method => {
            response.method = method::GET_SNAPSHOT.to_owned();
        }
    }
}
