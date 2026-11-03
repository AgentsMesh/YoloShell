use crate::paths::identity::FileIdentity;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity, GUARDIAN_PROTOCOL_VERSION,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) const GUARDIAN_RECOVERY_METHOD: &str = "pty.custody.guardian.recover";
pub(super) const RECOVERY_DESCRIPTOR_COUNT: usize = 1;
const OPENING_DETAIL: &str = "invalid PTY custody Guardian recovery envelope";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardianRecoveryRequest {
    protocol_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) transfer_id: Uuid,
    pub(super) requester: CustodyHolderIdentity,
    pub(super) guardian: CustodyGuardianBinding,
    pub(super) runtime_file_identity: FileIdentity,
    pub(super) durable: CustodyDurableRecord,
    pub(super) durable_file_identity: FileIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GuardianRecoveryPhase {
    Prepared,
    Committed,
    Acknowledged,
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardianRecoveryReceipt {
    protocol_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    transfer_id: Uuid,
    phase: GuardianRecoveryPhase,
}

impl GuardianRecoveryRequest {
    pub(super) fn new(
        requester: CustodyHolderIdentity,
        guardian: CustodyGuardianBinding,
        runtime_file_identity: FileIdentity,
        durable: CustodyDurableRecord,
        durable_file_identity: FileIdentity,
    ) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            transfer_id: Uuid::new_v4(),
            requester,
            guardian,
            runtime_file_identity,
            durable,
            durable_file_identity,
        }
    }

    pub(super) fn decode(opening: &RequestEnvelope) -> Result<Self> {
        anyhow::ensure!(
            opening.version == crate::model::PROTOCOL_VERSION
                && opening.kind == crate::protocol::REQUEST_KIND
                && opening.method == GUARDIAN_RECOVERY_METHOD,
            OPENING_DETAIL
        );
        opening.decode_payload().map_err(Into::into)
    }

    pub(super) fn protocol_is_current(&self) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION
    }
}

impl GuardianRecoveryReceipt {
    pub(super) fn new(transfer_id: Uuid, phase: GuardianRecoveryPhase) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            transfer_id,
            phase,
        }
    }

    pub(super) fn validates(
        self,
        request: &GuardianRecoveryRequest,
        phase: GuardianRecoveryPhase,
    ) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION
            && self.transfer_id == request.transfer_id
            && self.phase == phase
    }
}
