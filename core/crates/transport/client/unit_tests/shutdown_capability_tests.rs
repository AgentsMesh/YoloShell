use super::*;
use crate::model::PROTOCOL_VERSION;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, capability, method};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::UnixListener;

const TEST_BOOT_ID: &str = "shutdown-capability-boot";
const TEST_HOST_PID: u32 = 41;
const TEST_CONTROLLER_GENERATION: u64 = 7;

#[tokio::test]
async fn unsupported_host_is_rejected_before_shutdown_request() {
    let server = CapabilityServer::start(Vec::new()).await;
    let client = Client::new(&server.socket);

    let error = client.shutdown_if_idle().await.unwrap_err();

    assert!(matches!(
        error,
        ClientError::HostCapabilityUnavailable { .. }
    ));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn advertised_idle_shutdown_is_requested_after_negotiation() {
    let server = CapabilityServer::start(vec![capability::IDLE_HOST_SHUTDOWN.into()]).await;
    let client = Client::new(&server.socket);

    client.shutdown_if_idle().await.unwrap();

    assert_eq!(
        server.finish().await,
        vec![method::HOST_STATUS, method::SHUTDOWN_IF_IDLE]
    );
}

struct CapabilityServer {
    _directory: tempfile::TempDir,
    socket: PathBuf,
    methods: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl CapabilityServer {
    async fn start(capabilities: Vec<String>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("host.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&methods);
        let home = directory.path().to_path_buf();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = crate::rpc_test_support::Peer::new(stream);
            while let Ok(request) = stream.request().await {
                recorded.lock().unwrap().push(request.method.clone());
                let response = response(&request, &home, capabilities.clone());
                if stream.respond(&response).await.is_err() {
                    break;
                }
            }
        });
        Self {
            _directory: directory,
            socket,
            methods,
            task,
        }
    }

    async fn finish(self) -> Vec<String> {
        self.task.await.unwrap();
        self.methods.lock().unwrap().clone()
    }
}

fn response(
    request: &RequestEnvelope,
    home: &std::path::Path,
    capabilities: Vec<String>,
) -> ResponseEnvelope {
    if request.method == method::HOST_STATUS {
        let status = HostStatus {
            protocol_version: PROTOCOL_VERSION,
            boot_id: TEST_BOOT_ID.into(),
            host_pid: TEST_HOST_PID,
            controller_generation: TEST_CONTROLLER_GENERATION,
            home: home.to_path_buf(),
            capabilities,
        };
        return ResponseEnvelope::success(request, &status).unwrap();
    }
    ResponseEnvelope::success(request, &()).unwrap()
}
