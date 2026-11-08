use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    fixtures::{constants::MESSAGE, target},
};
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, AgentWorkingDirectory, Lifecycle, ObservedProvider,
};
use yolo_hub_connector_protocol::{
    Activation, ConnectorScope, Enrollment, FIRST_PUBLICATION_SEQUENCE, Publication,
    PublicationAck, ReportedState,
};
use yolo_hub_identity::{ConnectorInstanceId, RuntimeTarget};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_runtime::{
    host::{ListSessionsRequest, ListSessionsResult},
    model::TerminalView,
    protocol::{ErrorBody, RequestEnvelope, ResponseEnvelope, error_code, method},
    worker::TerminalRef,
};

pub const PROVIDER: &str = "fixture-observation-connector";
pub const DIRECTORY: &str = "/connector reported directory ";
pub const PUBLICATION_STEP: u64 = 1;

pub fn scope(terminal: RuntimeTarget) -> ConnectorScope {
    ConnectorScope {
        terminal,
        provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
    }
}
pub fn publication(activation: Activation) -> Publication {
    Publication {
        connector_id: activation.connector_id,
        generation: activation.generation,
        agent: activation.agent,
        terminal: activation.terminal,
        sequence: FIRST_PUBLICATION_SEQUENCE,
        state: ReportedState {
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            working_directory: Some(AgentWorkingDirectory::new(DIRECTORY.into()).unwrap()),
        },
    }
}
pub fn get(request: &RequestEnvelope, view: &TerminalView) -> ResponseEnvelope {
    assert_eq!(request.method, method::GET_SESSION);
    let asked: TerminalRef = request.decode_payload().unwrap();
    assert_eq!(asked.terminal_id, view.manifest.terminal_id);
    ResponseEnvelope::success(request, view).unwrap()
}
pub fn list(
    request: &RequestEnvelope,
    views: Vec<TerminalView>,
    complete: bool,
) -> ResponseEnvelope {
    assert_eq!(request.method, method::LIST_SESSIONS);
    let asked: ListSessionsRequest = request.decode_payload().unwrap();
    assert!(asked.require_complete);
    ResponseEnvelope::success(
        request,
        &ListSessionsResult {
            sessions: views,
            complete,
        },
    )
    .unwrap()
}
pub fn unavailable(request: &RequestEnvelope) -> ResponseEnvelope {
    assert!(matches!(
        request.method.as_str(),
        method::GET_SESSION | method::LIST_SESSIONS
    ));
    ResponseEnvelope::failure(
        request,
        ErrorBody::new(error_code::WORKER_UNREACHABLE, MESSAGE, false),
    )
}
pub async fn register(fixture: &ProtocolFixture, view: &TerminalView) -> Enrollment {
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture
                .backend
                .register_connector(uuid::Uuid::new_v4(), scope(target())),
            fixture.reply(|request| get(request, view))
        )
    })
    .await;
    result.unwrap()
}
pub async fn activate(
    fixture: &ProtocolFixture,
    enrollment: &Enrollment,
    view: &TerminalView,
) -> Activation {
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.activate_connector(
                enrollment.connector_id,
                enrollment.credential,
                ConnectorInstanceId::new()
            ),
            fixture.reply(|request| get(request, view))
        )
    })
    .await;
    result.unwrap()
}
pub async fn publish(
    fixture: &ProtocolFixture,
    activation: Activation,
    view: &TerminalView,
) -> PublicationAck {
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture
                .backend
                .publish_connector(activation, publication(activation)),
            fixture.reply(|request| get(request, view))
        )
    })
    .await;
    result.unwrap()
}
pub async fn observe(fixture: &ProtocolFixture, views: Vec<TerminalView>) -> Vec<AgentSnapshot> {
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            fixture.reply(|request| list(request, views, true))
        )
    })
    .await;
    result.unwrap()
}
pub async fn session(fixture: &ProtocolFixture, view: &TerminalView) -> TerminalSummary {
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.session(target().address),
            fixture.reply(|request| get(request, view))
        )
    })
    .await;
    result.unwrap()
}
