use crate::remote_broker::*;
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use tokio::{net::UnixListener, sync::oneshot};

const SOCKET: &str = "hub.sock";
const RETAINED_SOCKET: &str = "retained.sock";
const MODE: u32 = 0o600;
const OFFSET: u64 = 1;

#[derive(Clone, Copy)]
pub enum Fault {
    Request,
    Version,
    Instance,
    Connection,
    Generation,
    Profile,
    Peer,
    Endpoint,
    Result,
    Namespace,
    Deadline,
}

pub struct Server {
    _root: tempfile::TempDir,
    pub path: PathBuf,
    pub task: tokio::task::JoinHandle<()>,
    release: Option<oneshot::Sender<()>>,
}

impl Server {
    pub fn new(fault: Fault) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(SOCKET);
        let retained = root.path().join(RETAINED_SOCKET);
        let listener = UnixListener::bind(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(MODE)).unwrap();
        let server_path = path.clone();
        let (release, receive) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let envelope: BrokerRequestEnvelope =
                crate::protocol::read_frame(&mut stream).await.unwrap();
            if matches!(fault, Fault::Deadline) {
                let _ = receive.await;
                return;
            }
            let BrokerRequest::OpenHubPeer(mut request) = envelope.request else {
                panic!("expected Hub open")
            };
            match fault {
                Fault::Instance => request.binding.broker_instance_id = uuid::Uuid::new_v4(),
                Fault::Connection => request.binding.connection_id = uuid::Uuid::new_v4(),
                Fault::Generation => request.binding.connection_generation += OFFSET,
                Fault::Profile => request.binding.profile.host_id.push('x'),
                Fault::Peer => request.peer = yolo_broker_hub_test_support::request().peer,
                Fault::Endpoint => request.socket = super::hub_fixture::WRONG_SOCKET.into(),
                _ => {}
            }
            let opened = BrokerHubOpened {
                request,
                channel_id: yolo_broker_hub_test_support::CHANNEL,
            };
            let result = if matches!(fault, Fault::Result) {
                BrokerResult::HubTransport(opened.request.binding)
            } else {
                BrokerResult::HubPeer(opened)
            };
            let mut response = BrokerResponse::success(envelope.request_id, result);
            if matches!(fault, Fault::Request) {
                response.request_id = uuid::Uuid::new_v4();
            }
            if matches!(fault, Fault::Version) {
                response.protocol_version += OFFSET as u16;
            }
            let _replacement = if matches!(fault, Fault::Namespace) {
                std::fs::rename(&server_path, retained).unwrap();
                let replacement = UnixListener::bind(&server_path).unwrap();
                std::fs::set_permissions(&server_path, std::fs::Permissions::from_mode(MODE))
                    .unwrap();
                Some(replacement)
            } else {
                None
            };
            crate::protocol::write_frame(&mut stream, &response)
                .await
                .unwrap();
            let _ = receive.await;
        });
        Self {
            _root: root,
            path,
            task,
            release: Some(release),
        }
    }
    pub async fn finish(mut self) {
        let _ = self.release.take().unwrap().send(());
        (&mut self.task).await.unwrap();
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
