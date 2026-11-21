use super::{cleanup_rpc::{ProtocolFixture, bounded}, native_support::{catalogue, source}};
use std::time::Duration;
use yolo_hub_agent_protocol::Capability;

const OBSERVATION_BUDGET_SLACK: Duration = Duration::from_secs(2);

/// One Worker that never answers its observation must not hold the catalogue
/// read open, and its agent keeps its row without submit authority.
#[tokio::test]
async fn a_stalled_worker_observation_is_bounded() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let mut sweep = Box::pin(fixture.backend.observe_agents());
    let started = tokio::time::Instant::now();
    // The Host catalogue is answered; the observation request that follows is
    // never accepted, so only the observation budget can end the sweep.
    let (agents, ()) = bounded(async {
        tokio::join!(
            sweep.as_mut(),
            fixture.reply(|request| catalogue(request, &view))
        )
    })
    .await;
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
