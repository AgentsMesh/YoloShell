use crate::invites::InviteLedgerError;
use crate::{ManagementService, clock::wall_clock_ms};
use yolo_hub_enrollment_protocol::{InvitePolicy, SignedInvite};
use yolo_hub_enrollment_wire::{JOIN_SCHEMA_VERSION, JoinErrorCode, JoinReply, JoinRequest};
use yolo_hub_enrollment_server::EnrollmentAdmission;
use yolo_hub_federation_settings::{ExportPolicy, Peer};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_routing::Right;
use yolo_hub_terminal_protocol::PublicError;

impl ManagementService {
    pub(crate) async fn admit_enrollment(&mut self, admission: EnrollmentAdmission) {
        let result = self.enroll(admission.invite, admission.request).await;
        let _ = admission.reply.send(result);
    }

    async fn enroll(
        &mut self,
        invite: SignedInvite,
        request: JoinRequest,
    ) -> Result<JoinReply, JoinErrorCode> {
        request.validate()?;
        if request.invite_id != invite.claims().invite_id {
            return Err(JoinErrorCode::Unauthorized);
        }
        let now_ms = wall_clock_ms().map_err(|_| JoinErrorCode::Unavailable)?;
        invite
            .verify_at(now_ms)
            .map_err(|_| JoinErrorCode::Unauthorized)?;
        let local = self.hub.state.peer_identity();
        if invite.claims().issuer != local {
            return Err(JoinErrorCode::Unauthorized);
        }
        if request.peer.hub() == local.hub() || request.peer.key() == local.key() {
            return Err(JoinErrorCode::Conflict);
        }
        let policy = invite.claims().policy;
        let exports = exports(local.hub(), policy);
        if let Some(peer) = self.configuration.configuration.peer(request.peer.hub()) {
            let same = peer.identity == request.peer
                && peer.dial.is_none()
                && peer.consume == policy.import_peer
                && peer.exports == exports;
            if !same {
                return Err(JoinErrorCode::Conflict);
            }
            // Burn a second valid code even though this peer is already
            // configured; an already-consumed code still recovers idempotently.
            match self.invites.consume(
                invite.claims().invite_id,
                invite.claims().expires_at_ms,
                now_ms,
            ) {
                Ok(()) | Err(InviteLedgerError::Consumed) => {}
                Err(InviteLedgerError::Full | InviteLedgerError::Unavailable) => {
                    return Err(JoinErrorCode::Unavailable)
                }
            }
            let snapshot = self.snapshot().map_err(|_| JoinErrorCode::Unavailable)?;
            return Ok(reply(snapshot));
        }
        self.invites
            .consume(
                invite.claims().invite_id,
                invite.claims().expires_at_ms,
                now_ms,
            )
            .map_err(|error| match error {
                InviteLedgerError::Consumed => JoinErrorCode::Unauthorized,
                InviteLedgerError::Full | InviteLedgerError::Unavailable => {
                    JoinErrorCode::Unavailable
                }
            })?;
        let mut configuration = self.configuration.configuration.clone();
        configuration.peers.push(Peer {
            identity: request.peer,
            dial: None,
            consume: policy.import_peer,
            exports,
        });
        configuration
            .validate(local)
            .map_err(|_| JoinErrorCode::InvalidRequest)?;
        let change = ConfigurationChange {
            request_id: request.request_id,
            expected_revision: self.configuration.revision,
            configuration,
        };
        match self.change(change).await {
            Ok(snapshot) => Ok(reply(snapshot)),
            Err(PublicError::OutcomeUnknown)
                if self
                    .configuration
                    .configuration
                    .peer(request.peer.hub())
                    .is_some_and(|peer| peer.identity == request.peer) =>
            {
                Ok(JoinReply {
                    schema_version: JOIN_SCHEMA_VERSION,
                    hub: local,
                    revision: self.configuration.revision,
                    applied: false,
                })
            }
            Err(error) => Err(map_error(error)),
        }
    }
}

fn reply(snapshot: ConfigurationSnapshot) -> JoinReply {
    JoinReply {
        schema_version: JOIN_SCHEMA_VERSION,
        hub: snapshot.state.identity,
        revision: snapshot.state.revision,
        applied: snapshot.applied,
    }
}

fn exports(local: HubId, policy: InvitePolicy) -> Vec<ExportPolicy> {
    if policy.export_local {
        vec![ExportPolicy {
            origin: local,
            rights: [Right::Observe].into_iter().collect(),
            remaining_reexports: 0,
        }]
    } else {
        Vec::new()
    }
}

fn map_error(error: PublicError) -> JoinErrorCode {
    match error {
        PublicError::Conflict => JoinErrorCode::Conflict,
        PublicError::InvalidRequest | PublicError::InvalidPayload => {
            JoinErrorCode::InvalidRequest
        }
        PublicError::Unsupported => JoinErrorCode::Unsupported,
        _ => JoinErrorCode::Unavailable,
    }
}
