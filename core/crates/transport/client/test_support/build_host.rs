use crate::{
    Client, HostStatus,
    protocol::{RequestEnvelope, ResponseEnvelope, method},
};
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;
use yolo_build_image::{BuildImage, SHA256_HEX_BYTES};

const SOCKET_NAME: &str = "host.sock";
const BOOT_ID: &str = "build-image-test-boot";
const HOST_PID: u32 = 41;
const GENERATION: u64 = 7;
const IMAGE_SIZE: u64 = 4096;
const DIGEST_CHARACTER: &str = "a";

pub fn image() -> BuildImage {
    BuildImage::new(DIGEST_CHARACTER.repeat(SHA256_HEX_BYTES), IMAGE_SIZE).unwrap()
}

pub struct Server {
    directory: tempfile::TempDir,
    socket: PathBuf,
    task: tokio::task::JoinHandle<Vec<String>>,
}
impl Server {
    pub async fn start(home: &Path, advertised: bool, valid: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let status = HostStatus {
            protocol_version: crate::model::PROTOCOL_VERSION,
            boot_id: BOOT_ID.into(),
            host_pid: HOST_PID,
            controller_generation: GENERATION,
            home: home.to_path_buf(),
            capabilities: if advertised {
                vec![crate::protocol::capability::HOST_BUILD.into()]
            } else {
                vec![]
            },
        };
        let task = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut connection = crate::rpc_test_support::Peer::new(connection);
            let mut methods = Vec::new();
            while let Ok(request) = connection.request().await {
                methods.push(request.method.clone());
                let response = match request.method.as_str() {
                    method::HOST_STATUS => ResponseEnvelope::success(&request, &status).unwrap(),
                    method::HOST_BUILD if valid => {
                        ResponseEnvelope::success(&request, &image()).unwrap()
                    }
                    method::HOST_BUILD => ResponseEnvelope::success(&request, &()).unwrap(),
                    _ => panic!("unexpected request"),
                };
                if connection.respond(&response).await.is_err() {
                    break;
                }
            }
            methods
        });
        Self {
            directory,
            socket,
            task,
        }
    }
    pub fn client(&self, home: Option<&Path>) -> Client {
        match home {
            Some(home) => Client::managed_local(home, &self.socket).unwrap(),
            None => Client::new(&self.socket),
        }
    }
    pub async fn finish(self) -> Vec<String> {
        let methods = self.task.await.unwrap();
        drop(self.directory);
        methods
    }
}
