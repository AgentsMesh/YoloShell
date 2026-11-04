use super::{
    cleanup_rpc::*,
    native_support::*,
};
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::{Capability, Lifecycle, Reachability};
use yolo_hub_rpc_protocol::Reply;
use yolo_runtime::{
    agent::AgentState,
    host::{ListSessionsRequest, ListSessionsResult},
    model::TerminalView,
    protocol::{RequestEnvelope, ResponseEnvelope, method},
};
use yolo_runtime_protocol::TerminalRef;

const SECOND_AGENT: usize = 2;

fn idle_view(native_reference: &str) -> TerminalView {
    let mut view = source();
    let terminal = uuid::Uuid::new_v4();
    view.manifest.terminal_id = terminal;
    let runtime = view.runtime.as_mut().unwrap();
    runtime.runtime_epoch = uuid::Uuid::new_v4();
    runtime.worker_id = uuid::Uuid::new_v4();
    let binding = view.agent.as_mut().unwrap();
    binding.terminal_id = terminal;
    binding.state = AgentState::WaitingForInput;
    binding.provider_session_id = native_reference.into();
    binding.evidence_path = format!("/evidence/{native_reference}");
    binding.data_home = format!("/home/{native_reference}");
    view
}

fn idle_observation(view: &TerminalView, revision: u64) -> NativeControlObservation {
    let unready = observation(view, revision);
    NativeControlObservation::new(unready.binding().clone(), unready.witness().clone(), true).unwrap()
}

fn listing(request: &RequestEnvelope, views: &[TerminalView]) -> ResponseEnvelope {
    assert_eq!(request.method, method::LIST_SESSIONS);
    assert!(
        !request
            .decode_payload::<ListSessionsRequest>()
            .unwrap()
            .include_archived
    );
    ResponseEnvelope::success(
        request,
        &ListSessionsResult {
            sessions: views.to_vec(),
            complete: true,
        },
    )
    .unwrap()
}

fn session(request: &RequestEnvelope, view: &TerminalView) -> ResponseEnvelope {
    assert_eq!(request.method, method::GET_SESSION);
    let asked: TerminalRef = request.decode_payload().unwrap();
    assert_eq!(asked.terminal_id, view.manifest.terminal_id);
    ResponseEnvelope::success(request, view).unwrap()
}

/// A catalogue sweep over N idle agents issues no Worker observation once
/// each binding is stored: the cost is one Host catalogue read.
#[tokio::test]
async fn idle_catalogue_sweep_issues_no_worker_calls() {
    let fixture = ProtocolFixture::new().with_agents();
    let first = idle_view("native-first");
    let second = idle_view("native-second");
    let observed = [
        idle_observation(&first, SOURCE_REVISION),
        idle_observation(&second, SOURCE_REVISION),
    ];
    let views = [first.clone(), second.clone()];
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            async {
                fixture.reply(|request| listing(request, &views)).await;
                fixture
                    .reply(|request| native(request, &first, Some(&observed[0])))
                    .await;
                fixture
                    .reply(|request| native(request, &second, Some(&observed[1])))
                    .await;
            }
        )
    })
    .await;
    let agents = agents.unwrap();
    assert_eq!(agents.len(), SECOND_AGENT);
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            fixture.reply(|request| listing(request, &views))
        )
    })
    .await;
    let agents = agents.unwrap();
    assert_eq!(agents.len(), SECOND_AGENT);
    assert!(agents.iter().all(|agent| matches!(
        agent.fields().lifecycle,
        Lifecycle::Running {}
    )));
    assert!(agents.iter().all(|agent| {
        agent.fields().capabilities.submit_prompt == Capability::Available {}
    }));
    fixture.assert_no_pending_connection();
}

/// A get of an unchanged idle agent reads its session and nothing else; it
/// never reads PTY output, screen snapshots or journal bytes.
#[tokio::test]
async fn idle_get_reads_only_its_session_view() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = idle_view("native-idle");
    let observed = idle_observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    let target = snapshot.cursor().target();
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.get_agent(target),
            fixture.reply(|request| session(request, &view))
        )
    })
    .await;
    let Reply::Agent { agent } = reply.unwrap() else {
        panic!("expected an agent reply")
    };
    assert_eq!(agent.cursor().target(), target);
    assert_eq!(
        agent.fields().capabilities.submit_prompt,
        Capability::Available {}
    );
    fixture.assert_no_pending_connection();
}

/// An archived session is local history. Reading the complete catalogue must
/// not revive it into an observation the live-only sweep then contradicts;
/// a steady catalogue must leave the persisted projection record untouched.
#[tokio::test]
async fn archived_history_does_not_rewrite_the_projection_record() {
    async fn cycle(fixture: &ProtocolFixture, archived: &TerminalView) {
        let (terminals, ()) = bounded(async {
            tokio::join!(
                fixture.backend.terminals(true),
                fixture.reply(|request| archived_listing(
                    request,
                    std::slice::from_ref(archived)
                ))
            )
        })
        .await;
        assert_eq!(terminals.unwrap().len(), 1);
        let (_, ()) = bounded(async {
            tokio::join!(
                fixture.backend.observe_agents(),
                fixture.reply(|request| live_listing(request, &[]))
            )
        })
        .await;
    }
    let fixture = ProtocolFixture::new().with_agents();
    let view = idle_view("native-archived");
    let observed = idle_observation(&view, SOURCE_REVISION);
    observe(&fixture, &view, Some(&observed)).await;
    let mut archived = view;
    archived.manifest.archived = true;
    cycle(&fixture, &archived).await;
    let settled = fixture.projection_record();
    cycle(&fixture, &archived).await;
    assert_eq!(
        fixture.projection_record(),
        settled,
        "an unchanged catalogue must not rewrite the projection record"
    );
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            fixture.reply(|request| live_listing(request, &[]))
        )
    })
    .await;
    let agents = agents.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].fields().reachability,
        Reachability::Unreachable,
        "archived history is not live"
    );
    fixture.assert_no_pending_connection();
}
