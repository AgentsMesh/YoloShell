use super::{constants::*, graph, observe, process::Publisher, requests, shell};
use crate::hub::federation_support::{POLL, TEMP_ROOT, deadline};
use yolo_hub_agent_protocol::{Freshness, ObservationSource};
use yolo_hub_connector_protocol::{Command, ConnectorError, EnrollmentStatus, Reply};
use yolo_hub_identity::ConnectorInstanceId;

#[tokio::test(flavor = "multi_thread")]
async fn real_native_hook_supersedes_connector_without_inheriting_publisher_pid() {
    let harness = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let node =
        graph::standalone(&harness, &root.path().canonicalize().unwrap().join(C_STATE)).await;
    let target = shell::create(node.running(), &harness).await;
    let enrollment = requests::enroll(&node, target).await;
    let mut publisher = Publisher::start(&node.socket, target.address.hub_id).await;
    let active =
        requests::activate_publisher(&mut publisher, &enrollment, ConnectorInstanceId::new()).await;
    requests::publish(
        &mut publisher,
        requests::publication(active, active.next_sequence),
    )
    .await;
    observe::agent(node.running(), enrollment.agent, Freshness::Current).await;
    let before = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    assert!(before.agent.is_none());
    shell::send(
        node.running(),
        target,
        &crate::agent_recovery::federation_agent_command(harness.home.path()),
    )
    .await;
    let native = tokio::time::timeout_at(deadline(), async {
        loop {
            let view = harness
                .client()
                .get_session(target.address.terminal_id.as_uuid())
                .await
                .unwrap();
            let bound = view.agent.as_ref().is_some_and(|agent| {
                agent.process_pid.is_some()
                    && agent.process_start_time.is_some()
                    && agent.recovery_availability
                        == yolo_runtime::agent::AgentRecoveryAvailability::Available
            });
            if let Some(agent) = node.running().agents().await.into_iter().find(|agent| {
                bound
                    && agent.fields().terminal == target
                    && agent.fields().source == ObservationSource::ProviderHook
                    && agent.fields().freshness == Freshness::Current
            }) {
                break agent;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    assert_ne!(native.cursor().target().address, enrollment.agent);
    let shell = observe::session(node.running(), target).await;
    assert_eq!(shell.fields().agent, Some(native.cursor().target()));
    let stale = observe::agent(node.running(), enrollment.agent, Freshness::Stale).await;
    assert_eq!(stale.cursor().target(), active.agent);
    assert_eq!(
        requests::status(&node, &enrollment).await.status,
        EnrollmentStatus::Superseded
    );
    assert_eq!(
        publisher
            .call(Command::Publish {
                publication: requests::publication(active, NEXT_SEQUENCE)
            })
            .await,
        Reply::Error {
            code: ConnectorError::StaleGeneration
        }
    );
    let current = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let runtime = current.runtime.as_ref().unwrap();
    let original = before.runtime.as_ref().unwrap();
    assert_eq!(runtime.runtime_epoch, original.runtime_epoch);
    assert_eq!(
        (runtime.worker_pid, runtime.worker_start_time),
        (original.worker_pid, original.worker_start_time)
    );
    let binding = current.agent.as_ref().unwrap();
    assert_eq!(binding.kind, yolo_runtime::agent::AgentKind::Codex);
    assert_ne!(binding.process_pid.unwrap(), publisher.pid());
    assert!(
        crate::ProcessIdentity {
            pid: binding.process_pid.unwrap(),
            start_time: binding.process_start_time.unwrap()
        }
        .matches()
    );
    publisher.stop().await;
    let after = observe::session(node.running(), target).await;
    assert_eq!(after.fields().agent, Some(native.cursor().target()));
    node.stop().await;
}
