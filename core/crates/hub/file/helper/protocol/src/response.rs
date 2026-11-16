use crate::{Command, FILE_HELPER_SCHEMA, Request, RootBinding};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_file_protocol::ReadFileResult;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Payload {
    Build { image: yolo_build_image::BuildImage },
    Inspected { bindings: Vec<RootBinding> },
    Read { file: ReadFileResult },
    Error { code: PublicError },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub origin: HubId,
    pub payload: Payload,
}

impl Response {
    pub fn new(request: &Request, payload: Payload) -> Self {
        Self {
            schema_version: FILE_HELPER_SCHEMA,
            request_id: request.fields().request_id,
            origin: request.fields().origin,
            payload,
        }
    }
    pub fn validate_for(&self, request: &Request) -> Result<(), PublicError> {
        if self.schema_version != FILE_HELPER_SCHEMA {
            return Err(PublicError::UnsupportedSchema);
        }
        if self.request_id != request.fields().request_id {
            return Err(PublicError::InvalidRequest);
        }
        if self.origin != request.fields().origin {
            return Err(PublicError::OriginMismatch);
        }
        match (&self.payload, &request.fields().command) {
            (Payload::Build { .. }, Command::Build {}) => Ok(()),
            (Payload::Error { .. }, _) => Ok(()),
            (Payload::Inspected { bindings }, Command::Inspect { roots }) => {
                if bindings.len() == roots.len()
                    && bindings
                        .iter()
                        .zip(roots)
                        .all(|(binding, root)| &binding.root == root)
                {
                    Ok(())
                } else {
                    Err(PublicError::TargetMismatch)
                }
            }
            (Payload::Read { file }, Command::Read { request, .. }) => file.validate_for(request),
            _ => Err(PublicError::InvalidPayload),
        }
    }
}
