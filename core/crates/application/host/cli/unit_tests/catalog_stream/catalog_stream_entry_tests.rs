use super::*;
use tokio::net::UnixListener;
use yolo_runtime::host::{ListSessionsRequest, ListSessionsResult};
use yolo_runtime::model::{HostStatus, PROTOCOL_VERSION};
use yolo_runtime::protocol::{ResponseEnvelope, method};
use yolo_runtime_rpc_testing::Peer;

const ENTRY_SOCKET_NAME: &str = "catalog-entry.sock";
const ENTRY_BOOT_ID: &str = "catalog-entry-boot";
const ENTRY_HOST_PID: u32 = 47_001;
const ENTRY_CONTROLLER_GENERATION: u64 = 47_002;
const ENTRY_RELATIVE_HOME: &str = "relative-catalog-home";
const ENTRY_UNUSED_SOCKET: &str = "/unused/catalog-entry.sock";

#[test]
fn managed_catalog_connection_rejects_a_relative_home() {
    let connection = CatalogConnection::managed(
        Paths::new(ENTRY_RELATIVE_HOME),
        PathBuf::from(ENTRY_UNUSED_SOCKET),
    );

    assert!(connection.is_err());
}

#[tokio::test]
async fn managed_catalog_connection_authenticates_the_existing_host_before_listing() {
    let directory = tempfile::tempdir().unwrap();
    yolo_runtime::paths::directory::PrivateDirectory::open_or_create_strict(directory.path())
        .unwrap();
    let paths = Paths::new(directory.path());
    let socket = directory.path().join(ENTRY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let home = directory.path().to_path_buf();
    let server = tokio::spawn(async move {
        let (probe, _) = listener.accept().await.unwrap();
        let mut probe = Peer::new(probe);
        respond_status(&mut probe, home.clone()).await;
        let (connection, _) = listener.accept().await.unwrap();
        let mut connection = Peer::new(connection);
        respond_status(&mut connection, home).await;
        let request = connection.request().await.unwrap();
        assert_eq!(request.method, method::LIST_SESSIONS);
        let payload: ListSessionsRequest = request.decode_payload().unwrap();
        assert!(!payload.include_archived);
        assert!(payload.require_complete);
        let response = ResponseEnvelope::success(
            &request,
            &ListSessionsResult {
                sessions: Vec::new(),
                complete: true,
            },
        )
        .unwrap();
        connection.respond(&response).await.unwrap();
        assert!(connection.request().await.is_err());
    });
    let mut connection = CatalogConnection::managed(paths, socket).unwrap();

    assert!(connection.list_sessions().await.unwrap().is_empty());
    drop(connection);
    server.await.unwrap();
}

async fn respond_status(stream: &mut Peer, home: PathBuf) {
    let request = stream.request().await.unwrap();
    assert_eq!(request.method, method::HOST_STATUS);
    let status = HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: ENTRY_BOOT_ID.to_owned(),
        host_pid: ENTRY_HOST_PID,
        controller_generation: ENTRY_CONTROLLER_GENERATION,
        home,
        capabilities: Vec::new(),
    };
    let response = ResponseEnvelope::success(&request, &status).unwrap();
    stream.respond(&response).await.unwrap();
}
