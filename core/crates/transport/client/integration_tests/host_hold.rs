use std::{path::Path, time::Duration};
use tokio::{net::UnixListener, sync::oneshot, time::timeout};
use yolo_client::{Client, ClientError};
use yolo_runtime::{
    model::{HostStatus, PROTOCOL_VERSION},
    protocol::{RequestEnvelope, ResponseEnvelope, method},
};

const SOCKET: &str = "host.sock";
const HOST_PID: u32 = 41;
const GENERATION: u64 = 7;
const BOOT_ID: &str = "host-hold-test";
const TIMEOUT: Duration = Duration::from_secs(5);
const WRONG_HOME: &str = "different-host";

fn status(home: &Path) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: BOOT_ID.into(),
        host_pid: HOST_PID,
        controller_generation: GENERATION,
        home: home.to_path_buf(),
        capabilities: Vec::new(),
    }
}

#[tokio::test]
async fn managed_and_unmanaged_holds_authenticate_and_release_the_same_connection() {
    for managed in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().canonicalize().unwrap();
        let socket = home.join(SOCKET);
        let listener = UnixListener::bind(&socket).unwrap();
        let observed_status = status(&home);
        let (seen, heartbeat) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = crate::rpc_test_support::Peer::new(stream);
            let first: RequestEnvelope = stream.request().await.unwrap();
            assert_eq!(first.method, method::HOST_STATUS);
            stream
                .respond(&ResponseEnvelope::success(&first, &observed_status).unwrap())
                .await
                .unwrap();
            let repeated: RequestEnvelope = stream.request().await.unwrap();
            assert_eq!(repeated.method, method::HOST_STATUS);
            stream
                .respond(&ResponseEnvelope::success(&repeated, &observed_status).unwrap())
                .await
                .unwrap();
            seen.send(()).unwrap();
            while let Ok(request) = stream.request().await {
                assert_eq!(request.method, method::HOST_STATUS);
                if stream
                    .respond(&ResponseEnvelope::success(&request, &observed_status).unwrap())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let client = if managed {
            Client::managed_local(&home, &socket).unwrap()
        } else {
            Client::new(&socket)
        };
        let hold = timeout(TIMEOUT, client.retain_host())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(hold.initial_host_pid(), HOST_PID);
        timeout(TIMEOUT, heartbeat).await.unwrap().unwrap();
        drop(hold);
        timeout(TIMEOUT, server).await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn hold_never_publishes_before_local_host_authentication_succeeds() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().canonicalize().unwrap();
    let socket = home.join(SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let wrong = status(&home.join(WRONG_HOME));
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request: RequestEnvelope = stream.request().await.unwrap();
        stream
            .respond(&ResponseEnvelope::success(&request, &wrong).unwrap())
            .await
            .unwrap();
        assert!(stream.request().await.is_err());
    });
    let result = Client::managed_local(&home, &socket)
        .unwrap()
        .retain_host()
        .await;
    assert!(matches!(result, Err(ClientError::HostHomeMismatch { .. })));
    timeout(TIMEOUT, server).await.unwrap().unwrap();
}
