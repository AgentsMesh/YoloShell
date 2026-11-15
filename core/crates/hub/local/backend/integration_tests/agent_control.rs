use super::{
    cleanup_rpc::*,
    fixtures::{agent_control::*, constants::MESSAGE},
};
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::AgentId;
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_runtime::{
    host::{ListSessionsRequest, ListSessionsResult},
    protocol::{ErrorBody, RequestEnvelope, ResponseEnvelope, error_code, method},
};
use yolo_runtime_protocol::TerminalRef;

fn catalogue(request: &RequestEnvelope) -> ResponseEnvelope {
    assert_eq!(request.method, method::LIST_SESSIONS);
    let fields: ListSessionsRequest = request.decode_payload().unwrap();
    assert!(!fields.include_archived);
    ResponseEnvelope::success(
        request,
        &ListSessionsResult {
            sessions: vec![host_view()],
            complete: true,
        },
    )
    .unwrap()
}

/// A scoped control reads its target session, not the catalogue.
fn session(request: &RequestEnvelope) -> ResponseEnvelope {
    assert_eq!(request.method, method::GET_SESSION);
    let fields: TerminalRef = request.decode_payload().unwrap();
    assert_eq!(fields.terminal_id, host_view().manifest.terminal_id);
    ResponseEnvelope::success(request, &host_view()).unwrap()
}

async fn observe(fixture: &ProtocolFixture) -> AgentSnapshot {
    let (agents, ()) =
        bounded(async { tokio::join!(fixture.backend.observe_agents(), fixture.reply(catalogue)) })
            .await;
    let mut agents = agents.unwrap();
    assert_eq!(agents.len(), SINGLE_AGENT);
    agents.pop().unwrap()
}

#[tokio::test]
async fn origin_control_resolves_from_host_observation_and_rejects_without_a_mutating_host_call() {
    let fixture = ProtocolFixture::new().with_agents();
    assert!(fixture.backend.supports(Method::AgentControl));
    let observed = observe(&fixture).await;
    let original = request(&observed);
    let command = Command::AgentControl {
        request: original.clone(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(fixture.backend.execute(&command), fixture.reply(session))
    })
    .await;
    assert_eq!(
        reply,
        Ok(Reply::AgentControl {
            receipt: CommandReceipt::rejected(&original, FailureCode::Unsupported),
        })
    );
    let mut fields = original.fields().clone();
    fields.target.address.agent_id = AgentId::new();
    fields.expected_cursor = ObservationCursor::new(
        fields.target,
        fields.expected_cursor.stream(),
        fields.expected_cursor.sequence(),
    )
    .unwrap();
    let missing = Command::AgentControl {
        request: AgentControlRequest::new(fields).unwrap(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(fixture.backend.execute(&missing), fixture.reply(session))
    })
    .await;
    assert_eq!(reply, Err(PublicError::NotFound));
    assert_eq!(observe(&fixture).await, observed);
}

#[tokio::test]
async fn lost_host_observation_rejects_control_and_retains_the_same_agent_as_stale() {
    let fixture = ProtocolFixture::new().with_agents();
    let observed = observe(&fixture).await;
    let request = request(&observed);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let peer = fixture.reply(|request| {
        assert_eq!(request.method, method::GET_SESSION);
        ResponseEnvelope::failure(
            request,
            ErrorBody::new(error_code::WORKER_UNREACHABLE, MESSAGE, false),
        )
    });
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    assert_eq!(
        reply,
        Ok(Reply::AgentControl {
            receipt: CommandReceipt::rejected(&request, FailureCode::PreconditionsChanged),
        })
    );
    let cache = fixture.backend.agents.as_ref().unwrap().lock().await;
    let retained = cache
        .get(observed.cursor().target().address)
        .unwrap()
        .unwrap();
    assert_eq!(retained.cursor().target(), observed.cursor().target());
    assert_eq!(retained.fields().terminal, observed.fields().terminal);
    assert_eq!(retained.fields().freshness, Freshness::Stale);
    assert_eq!(retained.fields().reachability, Reachability::Unreachable);
    assert_eq!(retained.fields().lifecycle, observed.fields().lifecycle);
}
