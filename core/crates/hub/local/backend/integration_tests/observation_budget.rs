use super::{cleanup_rpc::{ProtocolFixture, bounded}, native_support::{catalogue, source}};
use std::time::Duration;
use yolo_hub_agent_protocol::Capability;
use yolo_runtime::protocol::method;

const OBSERVATION_BUDGET_SLACK: Duration = Duration::from_secs(2);

/// One Worker that never answers its observation must not hold the catalogue
/// read open, and its agent keeps its row without submit authority.
#[tokio::test]
async fn a_stalled_worker_observation_is_bounded() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let mut sweep = Box::pin(fixture.backend.observe_agents());
    let stalled = async {
        fixture.reply(|request| catalogue(request, &view)).await;
        fixture.accept_request().await
    };
    let (_peer, request) = tokio::select! {
        incoming = stalled => incoming,
        _ = &mut sweep => panic!("catalogue completed before contacting the Worker"),
    };
    assert_eq!(request.method, method::OBSERVE_NATIVE_AGENT);
    // Hold the connection open without replying. Use the timer clock once the
    // observation is in flight, so runner scheduling is outside the measured budget.
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let agents = bounded(sweep).await;
    assert!(
        started.elapsed() < OBSERVATION_BUDGET_SLACK,
        "a stalled Worker observation must not hold the catalogue read open"
    );
    let agents = agents.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].fields().capabilities.submit_prompt,
        Capability::Unsupported {}
    );
}
