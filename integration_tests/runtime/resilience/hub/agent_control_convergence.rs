use super::super::super::federation_support::{POLL, RunningNode, deadline};
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};

pub async fn exact(node: &RunningNode, expected: &AgentSnapshot) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = node.agents().await;
            let [current] = agents.as_slice() else {
                panic!()
            };
            assert_eq!(current.cursor().target(), expected.cursor().target());
            assert_eq!(current.cursor().stream(), expected.cursor().stream());
            assert_eq!(current.fields().terminal, expected.fields().terminal);
            // Peer catalogues propagate asynchronously. An older revision may
            // catch up; a newer revision would violate this no-effect baseline.
            assert!(current.cursor().sequence() <= expected.cursor().sequence());
            if current.cursor() == expected.cursor()
                && current.fields().freshness == Freshness::Current
                && current.fields().reachability == Reachability::Reachable
            {
                assert_eq!(current, expected);
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}
