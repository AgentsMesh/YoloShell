use crate::{Selection, constants::*, validate_install_root};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_distribution_bundle::BundleDescriptor;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    try_from = "PublicationRequestFields",
    into = "PublicationRequestFields"
)]
pub struct PublicationRequest(PublicationRequestFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationRequestFields {
    pub schema_version: u16,
    pub install_root: PathBuf,
    pub operation_id: Uuid,
    pub stage: Uuid,
    pub descriptor: BundleDescriptor,
    #[serde(deserialize_with = "crate::wire::required_option")]
    pub expected: Option<Selection>,
}

impl PublicationRequest {
    pub fn new(
        install_root: impl Into<PathBuf>,
        operation_id: Uuid,
        stage: Uuid,
        descriptor: BundleDescriptor,
        expected: Option<Selection>,
    ) -> Result<Self> {
        PublicationRequestFields {
            schema_version: PUBLICATION_SCHEMA_VERSION,
            install_root: install_root.into(),
            operation_id,
            stage,
            descriptor,
            expected,
        }
        .try_into()
    }
    pub fn fields(&self) -> &PublicationRequestFields {
        &self.0
    }
    pub fn operation_id(&self) -> Uuid {
        self.0.operation_id
    }
    pub fn install_root(&self) -> &Path {
        &self.0.install_root
    }
    pub fn validate_root(&self, root: &Path) -> Result<()> {
        validate_install_root(root)?;
        ensure!(root == self.install_root(), INVALID_ROOT);
        Ok(())
    }
    pub fn stage(&self) -> Uuid {
        self.0.stage
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.0.descriptor
    }
    pub fn expected(&self) -> Option<&Selection> {
        self.0.expected.as_ref()
    }
    pub fn selected(&self) -> Result<Selection> {
        match self
            .expected()
            .filter(|previous| previous.operation_id() == self.operation_id())
        {
            Some(selected) => Ok(selected.clone()),
            None => Selection::next(
                self.expected(),
                self.operation_id(),
                self.descriptor().build_id(),
            ),
        }
    }
}

impl TryFrom<PublicationRequestFields> for PublicationRequest {
    type Error = anyhow::Error;
    fn try_from(fields: PublicationRequestFields) -> Result<Self> {
        validate_install_root(&fields.install_root)?;
        ensure!(
            fields.schema_version == PUBLICATION_SCHEMA_VERSION
                && !fields.operation_id.is_nil()
                && !fields.stage.is_nil(),
            INVALID_REQUEST
        );
        if let Some(previous) = fields
            .expected
            .as_ref()
            .filter(|previous| previous.operation_id() == fields.operation_id)
        {
            ensure!(
                previous.build_id() == fields.descriptor.build_id(),
                INVALID_REQUEST
            );
        }
        let request = Self(fields);
        request.selected()?;
        Ok(request)
    }
}

impl From<PublicationRequest> for PublicationRequestFields {
    fn from(value: PublicationRequest) -> Self {
        value.0
    }
}
