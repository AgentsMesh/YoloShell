use crate::{Selection, constants::*, validate_install_root};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yolo_distribution_bundle::BundleDescriptor;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "SelectedReleaseFields", into = "SelectedReleaseFields")]
pub struct SelectedRelease(SelectedReleaseFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedReleaseFields {
    pub selection: Selection,
    pub descriptor: BundleDescriptor,
    pub release_directory: PathBuf,
}

impl SelectedRelease {
    pub fn fields(&self) -> &SelectedReleaseFields {
        &self.0
    }
    pub fn selection(&self) -> &Selection {
        &self.0.selection
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.0.descriptor
    }
    pub fn release_directory(&self) -> &Path {
        &self.0.release_directory
    }
}

impl TryFrom<SelectedReleaseFields> for SelectedRelease {
    type Error = anyhow::Error;
    fn try_from(fields: SelectedReleaseFields) -> Result<Self> {
        ensure!(
            fields.selection.build_id() == fields.descriptor.build_id(),
            INVALID_INSPECTION
        );
        validate_install_root(&fields.release_directory)?;
        Ok(Self(fields))
    }
}
impl From<SelectedRelease> for SelectedReleaseFields {
    fn from(value: SelectedRelease) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "InspectionFields", into = "InspectionFields")]
pub struct InspectionReceipt(InspectionFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionFields {
    pub schema_version: u16,
    pub install_root: PathBuf,
    #[serde(deserialize_with = "crate::wire::required_option")]
    pub selected: Option<SelectedRelease>,
}

impl InspectionReceipt {
    pub fn new(root: &Path, selected: Option<(Selection, BundleDescriptor)>) -> Result<Self> {
        let selected = selected
            .map(|(selection, descriptor)| {
                SelectedReleaseFields {
                    selection,
                    release_directory: descriptor.release_directory(root),
                    descriptor,
                }
                .try_into()
            })
            .transpose()?;
        InspectionFields {
            schema_version: PUBLICATION_SCHEMA_VERSION,
            install_root: root.into(),
            selected,
        }
        .try_into()
    }
    pub fn fields(&self) -> &InspectionFields {
        &self.0
    }
    pub fn selected(&self) -> Option<&SelectedRelease> {
        self.0.selected.as_ref()
    }
    pub fn validate(&self, root: &Path) -> Result<()> {
        validate_install_root(root)?;
        ensure!(self.0.install_root == root, INVALID_INSPECTION);
        if let Some(selected) = &self.0.selected {
            ensure!(
                selected.release_directory() == selected.descriptor().release_directory(root),
                INVALID_INSPECTION
            );
        }
        Ok(())
    }
}

impl TryFrom<InspectionFields> for InspectionReceipt {
    type Error = anyhow::Error;
    fn try_from(fields: InspectionFields) -> Result<Self> {
        ensure!(
            fields.schema_version == PUBLICATION_SCHEMA_VERSION,
            INVALID_INSPECTION
        );
        let receipt = Self(fields);
        receipt.validate(&receipt.0.install_root)?;
        Ok(receipt)
    }
}
impl From<InspectionReceipt> for InspectionFields {
    fn from(value: InspectionReceipt) -> Self {
        value.0
    }
}
