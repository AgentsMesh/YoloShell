use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_distribution_publication_protocol::{PublicationReceipt, PublicationRequest};
use yolo_remote_provisioning_protocol::RemoteBootstrapReceipt;
use yolo_paths::identity::ObjectIdentity;
use yolo_remote_provisioning_protocol::{ProvisionPhase, ProvisionRequest, validate_publication};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub parent: ObjectIdentity,
    pub root: ObjectIdentity,
    pub entry: ObjectIdentity,
    pub request: ProvisionRequest,
    pub publication: PublicationRequest,
    pub phase: ProvisionPhase,
    #[serde(deserialize_with = "required_option")]
    pub published: Option<PublicationReceipt>,
    #[serde(deserialize_with = "required_option")]
    pub enrollment: Option<RemoteBootstrapReceipt>,
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    Deserialize::deserialize(deserializer)
}
impl Record {
    pub fn validate(&self) -> Result<()> {
        let fields = self.request.fields();
        ensure!(
            self.schema == SCHEMA
                && self.publication.operation_id() == fields.request_id
                && self.publication.stage() == fields.request_id
                && self.publication.descriptor() == fields.source.descriptor(),
            INVALID
        );
        self.publication.validate_root(&fields.install_root)?;
        match self.phase {
            ProvisionPhase::Prepared | ProvisionPhase::Publishing => ensure!(
                self.published.is_none() && self.enrollment.is_none(),
                INVALID
            ),
            ProvisionPhase::Published | ProvisionPhase::Enrolling => ensure!(
                self.published.is_some() && self.enrollment.is_none(),
                INVALID
            ),
            ProvisionPhase::Complete => ensure!(
                self.published.is_some() && self.enrollment.is_some(),
                INVALID
            ),
        }
        if let Some(published) = &self.published {
            validate_publication(&self.request, &self.publication, published)?;
        }
        if let Some(receipt) = &self.enrollment {
            receipt.validate_for(&fields.bootstrap)?;
        }
        Ok(())
    }
}
