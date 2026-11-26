use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_hub_file_protocol::{
    FileRootSummary, MAX_FILE_PATH_BYTES, MAX_FILE_ROOTS, ReadFileRequest, RelativeFilePath,
};
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootBinding {
    pub root: FileRootSummary,
    pub fingerprint: Sha256Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Build {},
    Inspect {
        roots: Vec<FileRootSummary>,
    },
    Read {
        binding: RootBinding,
        request: ReadFileRequest,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub origin: HubId,
    pub protected: Vec<String>,
    pub command: Command,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RequestFields", into = "RequestFields")]
pub struct Request(RequestFields);

impl Request {
    pub fn new(fields: RequestFields) -> Result<Self, PublicError> {
        if fields.schema_version != FILE_HELPER_SCHEMA {
            return Err(PublicError::UnsupportedSchema);
        }
        if fields.request_id.is_nil()
            || (fields.protected.is_empty() != matches!(fields.command, Command::Build {}))
            || fields.protected.len() > MAX_PROTECTED_PATHS
        {
            return Err(PublicError::InvalidRequest);
        }
        for path in &fields.protected {
            if path.len() > MAX_FILE_PATH_BYTES {
                return Err(PublicError::InvalidMetadata);
            }
            let relative = path
                .strip_prefix(ROOT_PREFIX)
                .ok_or(PublicError::InvalidMetadata)?;
            RelativeFilePath::new(relative.into())?;
        }
        match &fields.command {
            Command::Build {} => {}
            Command::Inspect { roots } => {
                if roots.len() > MAX_FILE_ROOTS {
                    return Err(PublicError::InvalidRequest);
                }
                let mut seen = BTreeSet::new();
                for root in roots {
                    let address = root.fields().target.address;
                    if address.hub_id != fields.origin {
                        return Err(PublicError::OriginMismatch);
                    }
                    if !seen.insert(address.root_id) {
                        return Err(PublicError::InvalidRequest);
                    }
                }
            }
            Command::Read { binding, request } => {
                if binding.root.fields().target.address.hub_id != fields.origin {
                    return Err(PublicError::OriginMismatch);
                }
                if binding.root.fields().target != request.fields().target {
                    return Err(PublicError::TargetMismatch);
                }
            }
        }
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &RequestFields {
        &self.0
    }
}
impl TryFrom<RequestFields> for Request {
    type Error = PublicError;
    fn try_from(value: RequestFields) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<Request> for RequestFields {
    fn from(value: Request) -> Self {
        value.0
    }
}
