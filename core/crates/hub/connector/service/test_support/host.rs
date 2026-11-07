use super::{constants::*, fixture::*, host_view};
use std::future::Future;
use tokio::{net::UnixListener, time::timeout_at};
use yolo_hub_agent_protocol::{AgentSnapshot, ObservedProvider};
use yolo_hub_connector_protocol::ConnectorScope;
use yolo_runtime::{
    host::{ListSessionsRequest, ListSessionsResult},
    model::TerminalView,
    protocol::{RequestEnvelope, ResponseEnvelope, method},
    worker::TerminalRef,
};

// Typed protocol peer only: these tests make no claims about real PTY custody.
pub struct Host {
    pub fixture: Fixture,
    pub scope: ConnectorScope,
    listener: UnixListener,
    view: TerminalView,
}

impl Host {
    pub fn new() -> Self {
        let fixture = Fixture::new();
        let listener = UnixListener::bind(
            fixture
                .root
                .path()
                .canonicalize()
                .unwrap()
                .join(HOST_SOCKET),
        )
        .unwrap();
        let target = fixture.activation().terminal;
        Self {
            fixture,
            scope: ConnectorScope {
                terminal: target,
                provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
            },
            listener,
            view: host_view::view(target),
        }
    }

    pub async fn accept(&self) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
        timeout_at(deadline(), async {
            let (stream, _) = self.listener.accept().await.unwrap();
            let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
            let hello = stream.request().await.unwrap();
            assert_eq!(hello.method, method::HOST_STATUS);
            hello.decode_payload::<()>().unwrap();
            let home = self
                .fixture
                .root
                .path()
                .canonicalize()
                .unwrap()
                .join(HOST_DIRECTORY);
            let response = ResponseEnvelope::success(&hello, &host_view::status(home)).unwrap();
            stream.respond(&response).await.unwrap();
            let request = stream.request().await.unwrap();
            (stream, request)
        })
        .await
        .unwrap()
    }

    pub async fn respond<F: Future>(&self, operation: F) -> F::Output {
        timeout_at(deadline(), async {
            let (result, ()) = tokio::join!(operation, self.reply());
            result
        })
        .await
        .unwrap()
    }

    async fn reply(&self) {
        let (mut stream, request) = self.accept().await;
        let response = match request.method.as_str() {
            method::GET_SESSION => {
                let asked: TerminalRef = request.decode_payload().unwrap();
                assert_eq!(asked.terminal_id, self.view.manifest.terminal_id);
                ResponseEnvelope::success(&request, &self.view).unwrap()
            }
            method::LIST_SESSIONS => {
                let asked: ListSessionsRequest = request.decode_payload().unwrap();
                assert!(asked.require_complete);
                ResponseEnvelope::success(
                    &request,
                    &ListSessionsResult {
                        sessions: vec![self.view.clone()],
                        complete: true,
                    },
                )
                .unwrap()
            }
            _ => panic!("{UNEXPECTED_HOST_METHOD}: {}", request.method),
        };
        stream.respond(&response).await.unwrap();
    }

    pub async fn snapshot(&self) -> AgentSnapshot {
        let agents = self
            .respond(self.fixture.backend.observe_agents())
            .await
            .unwrap();
        let [agent]: [_; 1] = agents.try_into().unwrap();
        agent
    }
}
