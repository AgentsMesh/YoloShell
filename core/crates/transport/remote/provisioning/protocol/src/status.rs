use crate::{PROVISION_SCHEMA_VERSION, ProvisionRequest, constants::INVALID};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_distribution_publication_protocol::{PublicationReceipt, PublicationRequest};
use crate::RemoteBootstrapReceipt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionPhase {
    Prepared,
    Publishing,
    Published,
    Enrolling,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProvisionOutcome {
    Absent {},
    Unknown {
        phase: ProvisionPhase,
    },
    Superseded {
        phase: ProvisionPhase,
    },
    Published {
        publication: PublicationRequest,
        receipt: PublicationReceipt,
    },
    Ready {
        publication: PublicationRequest,
        receipt: PublicationReceipt,
        enrollment: RemoteBootstrapReceipt,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProvisionStatusFields", into = "ProvisionStatusFields")]
pub struct ProvisionStatus(ProvisionStatusFields);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionStatusFields {
    pub schema_version: u16,
    pub request: ProvisionRequest,
    pub outcome: ProvisionOutcome,
}
impl ProvisionStatus {
    pub fn new(request: ProvisionRequest, outcome: ProvisionOutcome) -> Result<Self> {
        ProvisionStatusFields {
            schema_version: PROVISION_SCHEMA_VERSION,
            request,
            outcome,
        }
        .try_into()
    }
    pub fn fields(&self) -> &ProvisionStatusFields {
        &self.0
    }
    pub fn validate_for(&self, request: &ProvisionRequest) -> Result<()> {
        ensure!(&self.0.request == request, INVALID);
        Ok(())
    }
}
impl TryFrom<ProvisionStatusFields> for ProvisionStatus {
    type Error = anyhow::Error;
    fn try_from(fields: ProvisionStatusFields) -> Result<Self> {
        ensure!(fields.schema_version == PROVISION_SCHEMA_VERSION, INVALID);
        match &fields.outcome {
            ProvisionOutcome::Published {
                publication,
                receipt,
            }
            | ProvisionOutcome::Ready {
                publication,
                receipt,
                ..
            } => validate_publication(&fields.request, publication, receipt)?,
            _ => {}
        }
        if let ProvisionOutcome::Ready { enrollment, .. } = &fields.outcome {
            enrollment.validate_for(&fields.request.fields().bootstrap)?;
        }
        Ok(Self(fields))
    }
}
impl From<ProvisionStatus> for ProvisionStatusFields {
    fn from(value: ProvisionStatus) -> Self {
        value.0
    }
}

pub fn validate_publication(
    request: &ProvisionRequest,
    publication: &PublicationRequest,
    receipt: &PublicationReceipt,
) -> Result<()> {
    let fields = request.fields();
    ensure!(
        publication.operation_id() == fields.request_id
            && publication.stage() == fields.request_id
            && publication.descriptor() == fields.source.descriptor(),
        INVALID
    );
    publication.validate_root(&fields.install_root)?;
    receipt.validate(publication, &fields.install_root)
}
