use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableSnapshot, CustodyGuardianBinding, CustodyGuardianPreparation, CustodyOffer,
    CustodyRequest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::pty_custody::GUARDIAN_PROTOCOL_VERSION;
pub(crate) const GUARDIAN_PREPARE_METHOD: &str = "pty.custody.guardian.prepare";
const OPENING_DETAIL: &str = "invalid PTY custody guardian opening envelope";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardianBootstrap {
    pub(super) protocol_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    pub(super) guardian_id: Uuid,
    pub(super) request: CustodyRequest,
    pub(super) offer: CustodyOffer,
    pub(super) runtime_file_identity: FileIdentity,
    pub(super) durable_predecessor: CustodyDurableSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardianReady {
    pub(super) protocol_version: u16,
    pub(super) preparation: CustodyGuardianPreparation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuardianPrepareRequest {
    protocol_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    guardian_id: Uuid,
    request: CustodyRequest,
    offer: CustodyOffer,
    runtime_file_identity: FileIdentity,
    durable_predecessor: CustodyDurableSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuardianPrepared {
    protocol_version: u16,
    pub preparation: CustodyGuardianPreparation,
}

impl GuardianBootstrap {
    pub(super) fn new(
        guardian_id: Uuid,
        request: CustodyRequest,
        offer: CustodyOffer,
        runtime_file_identity: FileIdentity,
        durable_predecessor: CustodyDurableSnapshot,
    ) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            guardian_id,
            request,
            offer,
            runtime_file_identity,
            durable_predecessor,
        }
    }

    pub(super) fn is_well_formed(&self) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION
            && self.guardian_id != Uuid::nil()
            && self.request.is_well_formed()
            && self.offer.matches_request(&self.request)
    }
}

impl GuardianReady {
    pub(super) fn new(preparation: CustodyGuardianPreparation) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            preparation,
        }
    }

    pub(super) fn validates(&self, expected: CustodyGuardianBinding) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION && self.preparation.binding() == expected
    }
}

impl GuardianPrepareRequest {
    pub fn new(
        guardian_id: Uuid,
        request: CustodyRequest,
        offer: CustodyOffer,
        runtime_file_identity: FileIdentity,
        durable_predecessor: CustodyDurableSnapshot,
    ) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            guardian_id,
            request,
            offer,
            runtime_file_identity,
            durable_predecessor,
        }
    }

    pub(super) fn parts(
        self,
    ) -> (
        Uuid,
        CustodyRequest,
        CustodyOffer,
        FileIdentity,
        CustodyDurableSnapshot,
    ) {
        (
            self.guardian_id,
            self.request,
            self.offer,
            self.runtime_file_identity,
            self.durable_predecessor,
        )
    }

    pub(super) fn protocol_is_current(&self) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION
    }

    pub(super) fn decode(opening: &crate::protocol::RequestEnvelope) -> anyhow::Result<Self> {
        anyhow::ensure!(
            opening.version == crate::model::PROTOCOL_VERSION
                && opening.kind == crate::protocol::REQUEST_KIND
                && opening.method == GUARDIAN_PREPARE_METHOD,
            OPENING_DETAIL
        );
        opening.decode_payload().map_err(Into::into)
    }
}

impl GuardianPrepared {
    pub(super) fn new(preparation: CustodyGuardianPreparation) -> Self {
        Self {
            protocol_version: GUARDIAN_PROTOCOL_VERSION,
            preparation,
        }
    }

    pub(crate) fn is_current(&self) -> bool {
        self.protocol_version == GUARDIAN_PROTOCOL_VERSION
    }

    pub fn validates(&self, expected: CustodyGuardianBinding) -> bool {
        self.is_current() && self.preparation.binding() == expected
    }
}
