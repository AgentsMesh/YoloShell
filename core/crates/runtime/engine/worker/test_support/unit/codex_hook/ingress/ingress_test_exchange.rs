use super::IngressEndpoint;
use crate::agent::hooks::{EventKind, EventSequence, HookEnvelope, SessionStartSource};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake, CodexOutbox, CodexOutboxState};
use std::path::{Path, PathBuf};

use super::super::super::protocol::CodexHookAck;
use super::super::ingress_live_client::PendingPeerExchange;
use super::super::ingress_live_source::PeerEvidence;

pub(in crate::worker::codex_hook::tests) struct AcceptedIngressExchange {
    pub(in crate::worker::codex_hook::tests) ack: CodexHookAck,
    pub(in crate::worker::codex_hook::tests) occurrence: HookEnvelope,
}

pub(in crate::worker::codex_hook::tests) struct DurableRejectionExchange {
    pub(in crate::worker::codex_hook::tests) ack: CodexHookAck,
    pub(in crate::worker::codex_hook::tests) occurrence: HookEnvelope,
    pub(in crate::worker::codex_hook::tests) event: HookEnvelope,
}

pub(in crate::worker::codex_hook::tests) struct PendingIngressExchange {
    client: PendingPeerExchange,
    intake: CodexHookIntake,
    outbox: CodexOutbox,
    snapshot_path: PathBuf,
    occurrence: HookEnvelope,
    before_outbox: CodexOutboxState,
    before_snapshot: Option<Vec<u8>>,
}

impl IngressEndpoint {
    pub(in crate::worker::codex_hook::tests) async fn exchange(
        &self,
        socket: &Path,
        event: &HookEnvelope,
    ) -> CodexHookAck {
        self.exchange_detailed(socket, event).await.ack
    }

    pub(in crate::worker::codex_hook::tests) async fn exchange_detailed(
        &self,
        socket: &Path,
        event: &HookEnvelope,
    ) -> AcceptedIngressExchange {
        self.begin_exchange(socket, event).finish_accepted().await
    }

    pub(in crate::worker::codex_hook::tests) fn begin_exchange(
        &self,
        socket: &Path,
        event: &HookEnvelope,
    ) -> PendingIngressExchange {
        self.begin_exchange_with_evidence(socket, event, PeerEvidence::Direct)
    }

    pub(in crate::worker::codex_hook::tests) async fn exchange_rejected(
        &self,
        socket: &Path,
        event: &HookEnvelope,
        evidence: PeerEvidence,
    ) -> CodexHookAck {
        self.begin_exchange_with_evidence(socket, event, evidence)
            .finish_rejected()
            .await
    }

    pub(in crate::worker::codex_hook::tests) async fn retry_existing(
        &self,
        socket: &Path,
        occurrence: &HookEnvelope,
        evidence: PeerEvidence,
    ) -> CodexHookAck {
        PendingPeerExchange::begin(
            self.assets.live_source(),
            self.assets.hook_executable(),
            socket,
            occurrence,
            evidence,
        )
        .finish()
        .await
        .unwrap()
    }

    pub(in crate::worker::codex_hook::tests) async fn replay(
        &self,
        socket: &Path,
        accepted: &AcceptedIngressExchange,
    ) -> CodexHookAck {
        let before_outbox = self.outbox(&accepted.occurrence).load().unwrap();
        let artifacts = self.artifacts(&accepted.occurrence);
        let before_snapshot = read_optional(artifacts.snapshot_path());
        let client = PendingPeerExchange::begin(
            self.assets.live_source(),
            self.assets.hook_executable(),
            socket,
            &accepted.occurrence,
            PeerEvidence::Direct,
        );
        let ack = client.finish().await.unwrap();
        assert!(ack.accepted);
        assert_eq!(ack.code, crate::codex_hooks::ACK_CODE_REPLAY);
        assert_eq!(ack.event_id, Some(accepted.occurrence.event_id));
        assert_eq!(ack.sequence, accepted.ack.sequence);
        assert_replay_settled(
            before_outbox,
            self.outbox(&accepted.occurrence).load().unwrap(),
        );
        assert_eq!(read_optional(artifacts.snapshot_path()), before_snapshot);
        ack
    }

    pub(in crate::worker::codex_hook::tests) fn outbox(&self, event: &HookEnvelope) -> CodexOutbox {
        CodexOutbox::for_identity(self.artifacts(event).outbox_path(), &event.identity).unwrap()
    }

    fn begin_exchange_with_evidence(
        &self,
        socket: &Path,
        event: &HookEnvelope,
        evidence: PeerEvidence,
    ) -> PendingIngressExchange {
        let artifacts = self.artifacts(event);
        let intake =
            CodexHookIntake::for_identity(artifacts.intake_directory(), &event.identity).unwrap();
        let mut raw = event.clone();
        if let EventKind::SessionStart { source } = &mut raw.event {
            if *source == SessionStartSource::Startup {
                *source = SessionStartSource::Resume;
            }
        }
        raw.event_id = uuid::Uuid::nil();
        raw.sequence = EventSequence::ZERO;
        let occurrence = intake.persist_occurrence_for_test(raw).unwrap();
        let outbox = self.outbox(&occurrence);
        let before_outbox = outbox.load().unwrap();
        let snapshot_path = artifacts.snapshot_path();
        let before_snapshot = read_optional(&snapshot_path);
        let client = PendingPeerExchange::begin(
            self.assets.live_source(),
            self.assets.hook_executable(),
            socket,
            &occurrence,
            evidence,
        );
        PendingIngressExchange {
            client,
            intake,
            outbox,
            snapshot_path,
            occurrence,
            before_outbox,
            before_snapshot,
        }
    }

    fn artifacts(&self, event: &HookEnvelope) -> CodexHookArtifacts {
        CodexHookArtifacts::new(
            &self.paths,
            event.identity.terminal_id,
            event.identity.runtime_epoch,
            event.identity.invocation_id,
        )
        .unwrap()
    }
}

pub(super) fn read_optional(path: impl AsRef<Path>) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

fn assert_replay_settled(mut before: CodexOutboxState, mut after: CodexOutboxState) {
    assert!(after.occurrence_receipt_count_for_test() == usize::default());
    before.clear_occurrence_receipts_for_test();
    after.clear_occurrence_receipts_for_test();
    assert_eq!(after, before);
}

#[path = "ingress_test_exchange_assertions.rs"]
mod assertions;

#[path = "ingress_test_rejection.rs"]
mod rejection;

#[path = "ingress_test_post_enqueue_failure.rs"]
mod post_enqueue_failure;

#[path = "ingress_test_ack_cleanup.rs"]
mod ack_cleanup;
pub(in crate::worker::codex_hook::tests) use ack_cleanup::CleanupEvidence;

#[path = "ingress_test_pending_cleanup.rs"]
mod pending_cleanup;

#[path = "ingress_test_indeterminate.rs"]
mod indeterminate;
