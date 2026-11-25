use super::super::federation_support::{POLL, RunningNode, deadline};
use super::constants::COHERENT_WORKER_DEADLINE;
use crate::{Harness, ProcessIdentity};
use std::path::Path;
use yolo_hub_agent_protocol::{
    AgentSnapshot, Capability, Freshness, ObservationCursor, ObservationSource, Reachability,
};
use yolo_hub_identity::RuntimeTarget;
use yolo_runtime::{model::TerminalView, worker::RuntimeRef};

pub async fn current(
    node: &RunningNode,
    target: RuntimeTarget,
    previous: Option<ObservationCursor>,
) -> AgentSnapshot {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Some(agent) = node
                .agents()
                .await
                .into_iter()
                .find(|agent| agent.fields().terminal == target)
            {
                let fields = agent.fields();
                if fields.freshness == Freshness::Current
                    && fields.reachability == Reachability::Reachable
                    && fields.capabilities.interrupt == (Capability::Available {})
                    && previous.is_none_or(|cursor| cursor != agent.cursor())
                {
                    assert_eq!(fields.source, ObservationSource::ProviderHook);
                    assert_eq!(
                        fields.capabilities.respond_interaction,
                        Capability::Unsupported {}
                    );
                    return agent;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn coherent_worker(host: &Harness, target: RuntimeTarget) -> TerminalView {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let view = host
                .client()
                .get_session(target.address.terminal_id.as_uuid())
                .await
                .unwrap();
            let runtime = view.runtime.as_ref().unwrap();
            assert_eq!(runtime.runtime_epoch, target.runtime_epoch.as_uuid());
            // A transient unauthenticated Host view retains runtime identity but
            // advertises no capabilities. Only explicit advertisement is ready.
            let advertised = runtime.capabilities.as_ref().is_some_and(|capabilities| {
                capabilities.iter().any(|capability| {
                    capability == yolo_runtime::protocol::capability::NATIVE_AGENT_CONTROL
                })
            });
            if advertised && let Some(binding) = &view.agent {
                let digest = yolo_agent_control_contract::binding_digest(binding).unwrap();
                let observed = host
                    .client()
                    .native_agent_observation(
                        RuntimeRef::new(
                            target.address.terminal_id.as_uuid(),
                            target.runtime_epoch.as_uuid(),
                        ),
                        digest,
                    )
                    .await
                    .unwrap();
                if let Some(observed) = observed {
                    assert_eq!(observed.binding(), binding);
                    let witness = observed.witness().fields();
                    assert_eq!(witness.worker_id, runtime.worker_id);
                    assert_eq!(witness.binding_digest, digest);
                    assert_eq!(Some(witness.process_pid), binding.process_pid);
                    assert_eq!(Some(witness.process_start_time), binding.process_start_time);
                    return view;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .expect(COHERENT_WORKER_DEADLINE)
}

pub async fn count(path: &Path, expected: usize) {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let observed = std::fs::read(path).unwrap();
            assert!(observed.len() <= expected);
            if observed.len() == expected {
                return;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub async fn identities_unchanged(host: &Harness, target: RuntimeTarget, before: &TerminalView) {
    let after = host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let first = before.runtime.as_ref().unwrap();
    let current = after.runtime.as_ref().unwrap();
    assert_eq!(current.runtime_epoch, first.runtime_epoch);
    assert_eq!(current.worker_id, first.worker_id);
    assert_eq!(
        (current.worker_pid, current.worker_start_time),
        (first.worker_pid, first.worker_start_time)
    );
    assert_eq!(
        (current.child_pid, current.child_start_time),
        (first.child_pid, first.child_start_time)
    );
    let previous_agent = before.agent.as_ref().unwrap();
    let agent = after.agent.as_ref().unwrap();
    assert_eq!(agent.provider_session_id, previous_agent.provider_session_id);
    assert_eq!(
        (agent.process_pid, agent.process_start_time),
        (
            previous_agent.process_pid,
            previous_agent.process_start_time
        )
    );
    assert_eq!(after.manifest.archived, before.manifest.archived);
    assert_eq!(
        after.manifest.recovery_state,
        before.manifest.recovery_state
    );
    for (pid, start_time) in [
        (current.worker_pid, current.worker_start_time),
        (current.child_pid, current.child_start_time),
        (
            agent.process_pid.unwrap(),
            agent.process_start_time.unwrap(),
        ),
    ] {
        assert!(ProcessIdentity { pid, start_time }.matches());
    }
}
