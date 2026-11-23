use super::super::{
    federation_support::{RunningNode, deadline},
    multipath::io,
};
use super::durable::Durable;
use crate::{Harness, ProcessIdentity};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{AttachRequest, LeaseGrant, LeaseObservation, TerminalSnapshot};
use yolo_hub_terminal_client::Attachment;
use yolo_runtime::model::TerminalView;

pub struct Baseline {
    session: TerminalView,
    terminal: TerminalSnapshot,
    lease: LeaseObservation,
    durable: Durable,
}

pub async fn settle(node: &RunningNode, target: RuntimeTarget) {
    let mut output = io::attach(node, target, None).await;
    io::exchange(node, target, &mut output).await;
    output.detach(deadline()).await.unwrap();
}

impl Baseline {
    pub async fn capture(node: &RunningNode, harness: &Harness, grant: LeaseGrant) -> Self {
        let target = grant.fields().target;
        let lease = lease(node, target).await;
        assert_eq!(lease.fields().generation, grant.fields().generation);
        assert_eq!(
            lease.fields().holder_id,
            grant.fields().principal.client_id.to_string()
        );
        assert_eq!(lease.fields().expires_at_ms, grant.fields().expires_at_ms);
        let session = harness
            .client()
            .get_session(target.address.terminal_id.as_uuid())
            .await
            .unwrap();
        let durable = Durable::capture(harness, &session);
        Self {
            session,
            terminal: snapshot(node, target).await,
            lease,
            durable,
        }
    }
    pub async fn unchanged(&self, node: &RunningNode, harness: &Harness, target: RuntimeTarget) {
        assert_eq!(snapshot(node, target).await, self.terminal);
        let after = self.custody_unchanged(node, harness, target).await;
        assert_eq!(after.runtime, self.session.runtime);
    }
    pub async fn after_canary(&self, node: &RunningNode, harness: &Harness, target: RuntimeTarget) {
        let after = self.custody_unchanged(node, harness, target).await;
        let current = after.runtime.as_ref().unwrap();
        let mut expected = self.session.runtime.as_ref().unwrap().clone();
        assert!(current.last_output_sequence > expected.last_output_sequence);
        // Only the independently confirmed canary may advance terminal output.
        expected.last_output_sequence = current.last_output_sequence;
        assert_eq!(*current, expected);
        assert_eq!(
            snapshot(node, target).await.fields().size,
            self.terminal.fields().size
        );
    }
    async fn custody_unchanged(
        &self,
        node: &RunningNode,
        harness: &Harness,
        target: RuntimeTarget,
    ) -> TerminalView {
        assert_eq!(lease(node, target).await, self.lease);
        let after = harness
            .client()
            .get_session(target.address.terminal_id.as_uuid())
            .await
            .unwrap();
        assert!(!after.manifest.archived);
        assert_eq!(
            after.manifest.terminal_id,
            target.address.terminal_id.as_uuid()
        );
        assert_eq!(after.manifest.cwd, self.session.manifest.cwd);
        assert_eq!(
            after.manifest.recovery_state,
            self.session.manifest.recovery_state
        );
        assert_eq!(
            (after.manifest.cols, after.manifest.rows),
            (self.session.manifest.cols, self.session.manifest.rows)
        );
        assert_eq!(after.agent, self.session.agent);
        self.durable.unchanged(&harness.paths(), &after);
        let runtime = after.runtime.as_ref().unwrap();
        let agent = after.agent.as_ref().unwrap();
        for identity in [
            ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
            ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            },
            ProcessIdentity {
                pid: agent.process_pid.unwrap(),
                start_time: agent.process_start_time.unwrap(),
            },
        ] {
            assert!(identity.matches());
        }
        after
    }
}

async fn lease(node: &RunningNode, target: RuntimeTarget) -> LeaseObservation {
    let Reply::LeaseObservation { lease: Some(lease) } = node
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::ObserveLease { target },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    lease
}

async fn snapshot(node: &RunningNode, target: RuntimeTarget) -> TerminalSnapshot {
    let output = Attachment::open(
        node.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let snapshot = output.initial().fields().snapshot.clone().unwrap();
    output.detach(deadline()).await.unwrap();
    snapshot
}
