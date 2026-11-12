use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_distribution_bundle::SHA256_HEX_BYTES;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "SelectionFields", into = "SelectionFields")]
pub struct Selection(SelectionFields);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionFields {
    pub schema_version: u16,
    pub revision: u64,
    pub operation_id: Uuid,
    pub build_id: String,
}

impl Selection {
    pub fn fields(&self) -> &SelectionFields {
        &self.0
    }
    pub fn build_id(&self) -> &str {
        &self.0.build_id
    }
    pub fn operation_id(&self) -> Uuid {
        self.0.operation_id
    }
    pub fn revision(&self) -> u64 {
        self.0.revision
    }
    pub fn next(previous: Option<&Self>, operation_id: Uuid, build_id: &str) -> Result<Self> {
        let revision = previous.map_or(Ok(FIRST_REVISION), |value| {
            value
                .revision()
                .checked_add(REVISION_STEP)
                .ok_or_else(|| anyhow::anyhow!(REVISION_OVERFLOW))
        })?;
        SelectionFields {
            schema_version: SELECTION_SCHEMA_VERSION,
            revision,
            operation_id,
            build_id: build_id.into(),
        }
        .try_into()
    }
}

impl TryFrom<SelectionFields> for Selection {
    type Error = anyhow::Error;
    fn try_from(fields: SelectionFields) -> Result<Self> {
        ensure!(
            fields.schema_version == SELECTION_SCHEMA_VERSION
                && fields.revision >= FIRST_REVISION
                && !fields.operation_id.is_nil()
                && fields.build_id.len() == SHA256_HEX_BYTES
                && fields
                    .build_id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
            INVALID_SELECTION
        );
        Ok(Self(fields))
    }
}

impl From<Selection> for SelectionFields {
    fn from(value: Selection) -> Self {
        value.0
    }
}
