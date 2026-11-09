use super::constants::*;
use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::{Semaphore, oneshot},
    task::{JoinHandle, JoinSet},
};
use yolo_broker_hub_protocol::BrokerHubConnectionBinding;
use yolo_hub_remote_setup_protocol::RemoteSetupRequest;
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BrokerPaths, BrokerRequest, BrokerRequestEnvelope, BrokerResponse,
    BrokerResult,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verb {
    Prepare,
    Provision,
    Recover,
    Advance,
}
pub struct State {
    pub calls: Vec<Verb>,
    pub published: bool,
    pub blocked: bool,
}
pub struct Broker {
    pub paths: BrokerPaths,
    pub state: Arc<Mutex<State>>,
    pub entered: Arc<Semaphore>,
    pub release: Arc<Semaphore>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}
impl Broker {
    pub async fn start(path: &Path, published: bool, blocked: bool) -> Self {
        let paths = BrokerPaths::new(path).unwrap();
        let listener = UnixListener::bind(paths.management_socket()).unwrap();
        std::fs::set_permissions(
            paths.management_socket(),
            std::fs::Permissions::from_mode(SOCKET_MODE),
        )
        .unwrap();
        let state = Arc::new(Mutex::new(State {
            calls: Vec::new(),
            published,
            blocked,
        }));
        let entered = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        let (stopped, stop) = oneshot::channel();
        let task = tokio::spawn(serve(
            listener,
            state.clone(),
            entered.clone(),
            release.clone(),
            stop,
        ));
        Self {
            paths,
            state,
            entered,
            release,
            stop: Some(stopped),
            task: Some(task),
        }
    }
    pub fn count(&self, verb: Verb) -> usize {
        self.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|value| **value == verb)
            .count()
    }
    pub async fn wait_provision(&self) {
        tokio::time::timeout_at(deadline(), self.entered.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    }
    pub fn unblock(&self) {
        self.state.lock().unwrap().blocked = false;
        self.release.add_permits(ONE);
    }
    pub async fn finish(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.task.take().unwrap().await.unwrap();
    }
}
impl Drop for Broker {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
async fn serve(
    listener: UnixListener,
    state: Arc<Mutex<State>>,
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
    mut stop: oneshot::Receiver<()>,
) {
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut stop => break,
            Some(result) = tasks.join_next(), if !tasks.is_empty() => result.unwrap(),
            accepted = listener.accept() => {
                let (stream, _) = accepted.unwrap(); tasks.spawn(respond(stream, state.clone(), entered.clone(), release.clone()));
            }
        }
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}
async fn respond(
    mut stream: UnixStream,
    state: Arc<Mutex<State>>,
    entered: Arc<Semaphore>,
    release: Arc<Semaphore>,
) {
    let request: BrokerRequestEnvelope = yolo_wire::read_frame(&mut stream).await.unwrap();
    assert_eq!(request.protocol_version, BROKER_PROTOCOL_VERSION);
    let result = match request.request {
        BrokerRequest::PrepareHubTransport(profile) => {
            state.lock().unwrap().calls.push(Verb::Prepare);
            BrokerResult::HubTransport(BrokerHubConnectionBinding {
                profile,
                broker_instance_id: uuid::Uuid::new_v4(),
                connection_id: uuid::Uuid::new_v4(),
                connection_generation: GENERATION,
            })
        }
        request => {
            let (call, verb) = match request {
                BrokerRequest::ProvisionHub(call) => (call, Verb::Provision),
                BrokerRequest::RecoverHubProvision(call) => (call, Verb::Recover),
                BrokerRequest::AdvanceHubProvision(call) => (call, Verb::Advance),
                _ => panic!(),
            };
            call.validate().unwrap();
            let blocked = {
                let mut state = state.lock().unwrap();
                state.calls.push(verb);
                state.blocked && verb == Verb::Provision
            };
            if blocked {
                entered.add_permits(ONE);
                release.acquire().await.unwrap().forget();
            }
            let published = {
                let mut state = state.lock().unwrap();
                if verb == Verb::Advance {
                    state.published = false;
                }
                state.published
            };
            let request = RemoteSetupRequest {
                request: call.request,
                expected_configuration_revision:
                    yolo_hub_management_protocol::FIRST_CONFIGURATION_REVISION,
            };
            let status = if published {
                yolo_hub_remote_setup_protocol_test_support::fixture::published(&request)
            } else {
                yolo_hub_remote_setup_protocol_test_support::fixture::ready(&request)
            };
            BrokerResult::HubProvision(status)
        }
    };
    yolo_wire::write_frame(
        &mut stream,
        &BrokerResponse {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: request.request_id,
            result: Some(result),
            error_code: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
}
