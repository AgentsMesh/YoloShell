use super::{
    super::federation_support::{POLL, RunningNode, deadline},
    constants::*,
};
use std::collections::BTreeSet;
use tokio::time::{Instant, timeout_at};
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_identity::{AgentAddress, HubId, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn connections(node: &RunningNode, expected: &[HubId]) {
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    timeout_at(deadline(), async {
        loop {
            let actual: BTreeSet<_> = node.node.connected_peers().await.into_iter().collect();
            if actual == expected {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn stable_inventory(node: &RunningNode, address: AgentAddress, target: RuntimeTarget) {
    let until = Instant::now() + STABLE_DIRECTORY_WINDOW;
    timeout_at(deadline(), async {
        loop {
            let agents = node.agents().await;
            assert_eq!(agents.len(), SINGLE_ORIGIN_AGENT);
            let agent = agents.first().unwrap();
            assert_eq!(agent.cursor().target().address, address);
            assert_eq!(agent.fields().freshness, Freshness::Current);
            assert_eq!(agent.fields().reachability, Reachability::Reachable);
            let mut client = node.client().await;
            let Reply::Terminals {
                terminals,
                next: None,
            } = client
                .call(
                    node.node.hub_id(),
                    Command::List {
                        include_archived: false,
                        after: None,
                    },
                    deadline(),
                )
                .await
                .unwrap()
            else {
                panic!()
            };
            let selected = terminals
                .iter()
                .filter(|session| session.fields().address == target.address)
                .collect::<Vec<_>>();
            assert_eq!(selected.len(), SINGLE_ORIGIN_AGENT);
            assert_eq!(
                selected.first().unwrap().fields().runtime.unwrap().target,
                target
            );
            if Instant::now() >= until {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn custody(
    harness: &crate::Harness,
    target: RuntimeTarget,
    original: &yolo_runtime::model::TerminalView,
) {
    let current = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert!(!current.manifest.archived);
    let original_runtime = original.runtime.as_ref().unwrap();
    let current_runtime = current.runtime.as_ref().unwrap();
    assert_eq!(
        current_runtime.runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(current_runtime.worker_id, original_runtime.worker_id);
    assert_eq!(current_runtime.worker_pid, original_runtime.worker_pid);
    assert_eq!(
        current_runtime.worker_start_time,
        original_runtime.worker_start_time
    );
    assert_eq!(current_runtime.child_pid, original_runtime.child_pid);
    assert_eq!(
        current_runtime.child_start_time,
        original_runtime.child_start_time
    );
    let original_agent = original.agent.as_ref().unwrap();
    let current_agent = current.agent.as_ref().unwrap();
    assert_eq!(current_agent.process_pid, original_agent.process_pid);
    assert_eq!(
        current_agent.process_start_time,
        original_agent.process_start_time
    );
}
