use super::{
    super::federation_support::{POLL, RunningNode, deadline},
    constants::*,
};
use std::path::Path;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, ObservationSource, Reachability};
use yolo_hub_client::ClientError;
use yolo_hub_identity::{AgentTarget, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub async fn current(
    node: &RunningNode,
    target: RuntimeTarget,
    directory: &Path,
    previous: Option<&AgentSnapshot>,
) -> AgentSnapshot {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = node.agents().await;
            if let [agent] = agents.as_slice() {
                if agent.fields().terminal == target
                    && agent
                        .fields()
                        .working_directory
                        .as_ref()
                        .map(|cwd| cwd.as_str())
                        == directory.to_str()
                    && agent.fields().freshness == Freshness::Current
                    && agent.fields().reachability == Reachability::Reachable
                {
                    assert_eq!(agent.fields().source, ObservationSource::ProviderHook);
                    if let Some(previous) = previous {
                        assert_eq!(agent.cursor().target(), previous.cursor().target());
                        assert_eq!(agent.cursor().stream(), previous.cursor().stream());
                        assert!(agent.cursor().sequence() > previous.cursor().sequence());
                    }
                    return agent.clone();
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn exact_query(node: &RunningNode, expected: &AgentSnapshot) {
    let target = expected.cursor().target();
    let Reply::Agent { agent } = node
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::GetAgent {
                target,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(agent.cursor().target(), target);
    assert_eq!(
        agent.fields().working_directory,
        expected.fields().working_directory
    );
    assert_eq!(agent.fields().source, ObservationSource::ProviderHook);
}

pub async fn stale(node: &RunningNode, expected: &AgentSnapshot) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let agents = node.agents().await;
            assert_eq!(agents.len(), SINGLE_AGENT);
            let agent = agents.first().unwrap();
            assert_eq!(agent.cursor().target(), expected.cursor().target());
            assert_eq!(agent.fields().terminal, expected.fields().terminal);
            assert_eq!(
                agent.fields().working_directory,
                expected.fields().working_directory
            );
            assert_eq!(agent.fields().source, ObservationSource::ProviderHook);
            if agent.fields().freshness == Freshness::Stale
                && agent.fields().reachability == Reachability::Unreachable
            {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn no_file_grant(entry: &RunningNode, relay: &RunningNode, origin: &RunningNode) {
    let Reply::FileRoots {
        roots, next: None, ..
    } = origin
        .client()
        .await
        .call(
            origin.node.hub_id(),
            Command::ListFileRoots { after: None },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(roots.is_empty());
    for node in [entry, relay] {
        let result = node
            .client()
            .await
            .call(
                origin.node.hub_id(),
                Command::ListFileRoots { after: None },
                deadline(),
            )
            .await;
        assert!(matches!(
            result,
            Err(ClientError::Remote(PublicError::Unreachable))
        ));
    }
}

pub fn identities(before: &AgentSnapshot, after: &AgentSnapshot) {
    assert_eq!(before.cursor().target(), after.cursor().target());
    assert_eq!(before.cursor().stream(), after.cursor().stream());
    assert_eq!(before.fields().terminal, after.fields().terminal);
}

pub async fn runtime(
    harness: &crate::Harness,
    target: RuntimeTarget,
    agent: AgentTarget,
    directory: &Path,
    before: &yolo_runtime::model::TerminalView,
) {
    let current = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert_eq!(current.manifest.cwd, before.manifest.cwd);
    assert_eq!(
        current.runtime.as_ref().unwrap().runtime_epoch,
        target.runtime_epoch.as_uuid()
    );
    assert_eq!(
        current.runtime.as_ref().unwrap().worker_id,
        before.runtime.as_ref().unwrap().worker_id
    );
    let binding = current.agent.as_ref().unwrap();
    let original = before.agent.as_ref().unwrap();
    assert_eq!(binding.kind, yolo_runtime::agent::AgentKind::ClaudeCode);
    assert_eq!(binding.cwd, directory.to_str().unwrap());
    assert_eq!(binding.process_pid, original.process_pid);
    assert_eq!(binding.process_start_time, original.process_start_time);
    assert_eq!(binding.provider_session_id, original.provider_session_id);
    assert!(
        crate::ProcessIdentity {
            pid: original.process_pid.unwrap(),
            start_time: original.process_start_time.unwrap(),
        }
        .matches()
    );
    let worker = before.runtime.as_ref().unwrap();
    assert!(
        crate::ProcessIdentity {
            pid: worker.worker_pid,
            start_time: worker.worker_start_time,
        }
        .matches()
    );
    assert_eq!(agent.address.hub_id, target.address.hub_id);
}
