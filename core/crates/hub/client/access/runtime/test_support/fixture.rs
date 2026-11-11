use super::{backend::Backend, constants::*, model};
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf, sync::Arc};
use yolo_hub_client_access_protocol::AccessConfiguration;
use yolo_hub_client_access_runtime::ClientAccessRuntime;
use yolo_hub_client_access_store::CLIENT_ACCESS_DIRECTORY;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_state::HubState;

pub struct Fixture {
    pub runtime: Arc<ClientAccessRuntime>,
    pub backend: Arc<Backend>,
    pub state: Arc<HubState>,
    pub root: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(
            HubState::open(
                &root.path().join(HUB_DIRECTORY),
                &root.path().join(HOST_DIRECTORY),
            )
            .unwrap(),
        );
        let backend = Arc::new(Backend::new(state.hub_id()));
        let runtime = ClientAccessRuntime::open(
            state.clone(),
            backend.clone(),
            Some(model::configuration(state.hub_id())),
        )
        .unwrap();
        Self {
            runtime,
            backend,
            state,
            root,
        }
    }

    pub fn access_directory(&self) -> PathBuf {
        self.state
            .directory()
            .unwrap()
            .path()
            .join(CLIENT_ACCESS_DIRECTORY)
    }

    pub fn record(&self) -> PathBuf {
        self.access_directory().join(RECORD_FILE)
    }

    pub fn sibling(&self, name: &str) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(name)
    }

    pub async fn call(&self, command: Command) -> Result<Reply, PublicError> {
        call(self.runtime.as_ref(), command).await
    }

    pub async fn current(&self) -> AccessConfiguration {
        let Reply::ClientAccess { configuration } =
            self.call(Command::GetClientAccess {}).await.unwrap()
        else {
            panic!()
        };
        configuration
    }
}

pub async fn call(dispatcher: &dyn Dispatcher, command: Command) -> Result<Reply, PublicError> {
    let request = Request::new(dispatcher.hub_id(), command).unwrap();
    let Operation::Reply(reply) = dispatcher.dispatch(request.clone()).await? else {
        panic!()
    };
    Response::new(&request, dispatcher.hub_id(), reply.clone())
        .validate_for(&request)
        .unwrap();
    Ok(reply)
}

pub fn write_new(path: &std::path::Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
