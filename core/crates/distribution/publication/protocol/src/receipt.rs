use crate::{PublicationRequest, Selection, constants::*, validate_install_root};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "PublicationReceiptFields",
    into = "PublicationReceiptFields"
)]
pub struct PublicationReceipt(PublicationReceiptFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationReceiptFields {
    pub schema_version: u16,
    pub operation_id: Uuid,
    pub stage: Uuid,
    pub selection: Selection,
    pub release_directory: PathBuf,
}

impl PublicationReceipt {
    pub fn new(request: &PublicationRequest, root: &Path, selection: Selection) -> Result<Self> {
        request.validate_root(root)?;
        let receipt: Self = PublicationReceiptFields {
            schema_version: PUBLICATION_SCHEMA_VERSION,
            operation_id: request.operation_id(),
            stage: request.stage(),
            selection,
            release_directory: request.descriptor().release_directory(root),
        }
        .try_into()?;
        receipt.validate(request, root)?;
        Ok(receipt)
    }
    pub fn fields(&self) -> &PublicationReceiptFields {
        &self.0
    }
    pub fn selection(&self) -> &Selection {
        &self.0.selection
    }
    pub fn release_directory(&self) -> &Path {
        &self.0.release_directory
    }
    pub fn validate(&self, request: &PublicationRequest, root: &Path) -> Result<()> {
        request.validate_root(root)?;
        ensure!(
            self.0.operation_id == request.operation_id()
                && self.0.stage == request.stage()
                && self.0.selection == request.selected()?
                && self.0.release_directory == request.descriptor().release_directory(root),
            INVALID_RECEIPT
        );
        Ok(())
    }
}

impl TryFrom<PublicationReceiptFields> for PublicationReceipt {
    type Error = anyhow::Error;
    fn try_from(fields: PublicationReceiptFields) -> Result<Self> {
        ensure!(
            fields.schema_version == PUBLICATION_SCHEMA_VERSION
                && !fields.operation_id.is_nil()
                && !fields.stage.is_nil()
                && fields.selection.operation_id() == fields.operation_id,
            INVALID_RECEIPT
        );
        validate_install_root(&fields.release_directory)?;
        Ok(Self(fields))
    }
}

impl From<PublicationReceipt> for PublicationReceiptFields {
    fn from(value: PublicationReceipt) -> Self {
        value.0
    }
}
