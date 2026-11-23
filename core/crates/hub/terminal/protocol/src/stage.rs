use crate::{PublicError, validation};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::{Component, Path};
use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;

/// Maximum decoded size of one RPC frame. A transfer may span any number of frames.
pub const ATTACHMENT_CHUNK_BYTES: usize = 128 * 1024;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageAttachmentRequest {
    pub target: RuntimeTarget,
    pub upload_id: Uuid,
    pub name: String,
    pub offset: u64,
    #[serde(serialize_with = "encode_bytes", deserialize_with = "decode_bytes")]
    pub data: Vec<u8>,
    pub eof: bool,
}

impl StageAttachmentRequest {
    pub fn validate(&self) -> Result<(), PublicError> {
        validation::request_id(self.upload_id)?;
        let mut parts = Path::new(&self.name).components();
        if !matches!(parts.next(), Some(Component::Normal(name)) if name == self.name.as_str())
            || parts.next().is_some()
            || self.name.chars().any(char::is_control)
        {
            return Err(PublicError::InvalidMetadata);
        }
        if self.data.len() > ATTACHMENT_CHUNK_BYTES
            || (self.data.is_empty() && !self.eof)
            || self.offset.checked_add(self.data.len() as u64).is_none()
        {
            return Err(PublicError::InvalidPayload);
        }
        Ok(())
    }

    pub fn next_offset(&self) -> Result<u64, PublicError> {
        self.offset
            .checked_add(self.data.len() as u64)
            .ok_or(PublicError::InvalidPayload)
    }
}

impl std::fmt::Debug for StageAttachmentRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageAttachmentRequest")
            .field("target", &self.target)
            .field("upload_id", &self.upload_id)
            .field("offset", &self.offset)
            .field("bytes", &self.data.len())
            .field("eof", &self.eof)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedAttachment {
    pub target: RuntimeTarget,
    pub upload_id: Uuid,
    pub next_offset: u64,
    pub path: Option<String>,
}

impl StagedAttachment {
    pub fn matches(&self, request: &StageAttachmentRequest) -> bool {
        self.target == request.target
            && self.upload_id == request.upload_id
            && Some(self.next_offset) == request.next_offset().ok()
            && match &self.path {
                None => !request.eof,
                Some(path) => {
                    request.eof
                        && Path::new(path).is_absolute()
                        && Path::new(path).file_name() == Some(request.name.as_ref())
                        && !path.chars().any(char::is_control)
                }
            }
    }
}

fn encode_bytes<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(data))
}

fn decode_bytes<'de, D: Deserializer<'de>>(source: D) -> Result<Vec<u8>, D::Error> {
    let text = String::deserialize(source)?;
    if text.len() > ATTACHMENT_CHUNK_BYTES.div_ceil(3) * 4 {
        return Err(serde::de::Error::custom(PublicError::InvalidPayload));
    }
    let data = STANDARD.decode(text).map_err(serde::de::Error::custom)?;
    if data.len() > ATTACHMENT_CHUNK_BYTES {
        return Err(serde::de::Error::custom(PublicError::InvalidPayload));
    }
    Ok(data)
}
