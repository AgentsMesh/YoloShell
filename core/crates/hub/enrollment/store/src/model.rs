use crate::constants::INVALID;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::{
    CONFIGURATION_REVISION_STEP, ConfigurationChange, ConfigurationSnapshot,
};
use yolo_remote_provisioning_protocol::{RemoteBootstrapReceipt, RemoteBootstrapRequest};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedEnrollment {
    pub request: RemoteBootstrapRequest,
    pub peer: PeerIdentity,
    pub snapshot: ConfigurationSnapshot,
    #[serde(deserialize_with = "required_option")]
    pub change: Option<ConfigurationChange>,
}

impl PreparedEnrollment {
    pub fn validate(&self) -> Result<()> {
        self.snapshot.validate()?;
        let request = self.request.fields();
        ensure!(
            self.snapshot.applied
                && self.snapshot.state.identity == self.peer
                && request.expected_peer.is_none_or(|peer| peer == self.peer)
                && request.consumer.hub() != self.peer.hub()
                && request.consumer.key() != self.peer.key(),
            INVALID
        );
        let configuration = match &self.change {
            Some(change) => {
                change.validate(self.peer)?;
                ensure!(
                    change.request_id == request.request_id
                        && change.expected_revision == self.snapshot.state.revision
                        && change
                            .expected_revision
                            .checked_add(CONFIGURATION_REVISION_STEP)
                            .is_some(),
                    INVALID
                );
                &change.configuration
            }
            None => &self.snapshot.state.configuration,
        };
        let peer = configuration
            .peer(request.consumer.hub())
            .ok_or_else(|| anyhow::anyhow!(INVALID))?;
        let export = configuration
            .export(request.consumer.hub(), self.peer.hub())
            .ok_or_else(|| anyhow::anyhow!(INVALID))?;
        ensure!(
            peer.identity == request.consumer
                && export.rights == request.rights
                && export.remaining_reexports == request.remaining_reexports
                && configuration
                    .listen
                    .is_some_and(|address| address.ip().is_loopback()),
            INVALID
        );
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.change
            .as_ref()
            .map_or(self.snapshot.state.revision, |change| {
                change
                    .expected_revision
                    .saturating_add(CONFIGURATION_REVISION_STEP)
            })
    }
    pub fn applied_to(&self, current: &ConfigurationSnapshot) -> bool {
        current.validate().is_ok()
            && current.applied
            && current.state.identity == self.peer
            && match &self.change {
                Some(change) => {
                    current.state.revision == self.revision()
                        && current.state.last_change == change.request_id
                        && current.state.configuration == change.configuration
                }
                None => current.state == self.snapshot.state,
            }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRecord {
    pub prepared: PreparedEnrollment,
    #[serde(deserialize_with = "required_option")]
    pub completion: Option<RemoteBootstrapReceipt>,
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Deserialize::deserialize(deserializer)
}
impl EnrollmentRecord {
    pub fn validate(&self) -> Result<()> {
        self.prepared.validate()?;
        if let Some(receipt) = &self.completion {
            receipt.validate_for(&self.prepared.request)?;
            ensure!(
                receipt.fields().peer == self.prepared.peer
                    && receipt.fields().configuration_revision == self.prepared.revision(),
                INVALID
            );
        }
        Ok(())
    }
}
