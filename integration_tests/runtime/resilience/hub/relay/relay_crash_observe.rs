use super::{
    super::federation_support::{POLL, RunningNode, deadline},
    constants::*,
};
use yolo_hub_agent_protocol::{Freshness, Lifecycle, Reachability};
use yolo_hub_identity::{AgentTarget, RuntimeTarget};

pub async fn disconnected(node: &RunningNode, agent: AgentTarget, runtime: RuntimeTarget) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = node.agents().await;
            let retained = agents
                .iter()
                .filter(|entry| entry.cursor().target().address.hub_id == agent.address.hub_id)
                .collect::<Vec<_>>();
            assert_eq!(retained.len(), ORIGIN_AGENT_COUNT);
            let observed = retained.first().unwrap();
            assert_eq!(observed.cursor().target(), agent);
            assert_eq!(observed.fields().terminal, runtime);
            assert!(!matches!(
                observed.fields().lifecycle,
                Lifecycle::Exited { .. }
            ));
            if observed.fields().freshness == Freshness::Stale
                && observed.fields().reachability == Reachability::Unreachable
            {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn same_agent(node: &RunningNode, expected: AgentTarget, runtime: RuntimeTarget) {
    let agents = node.agents().await;
    let selected = agents
        .iter()
        .filter(|agent| agent.cursor().target().address == expected.address)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), ORIGIN_AGENT_COUNT);
    let agent = selected.first().unwrap();
    assert_eq!(agent.cursor().target(), expected);
    assert_eq!(agent.fields().terminal, runtime);
    assert_eq!(agent.fields().reachability, Reachability::Reachable);
    assert_eq!(agent.fields().freshness, Freshness::Current);
}
