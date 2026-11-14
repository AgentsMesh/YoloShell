use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::WorkspaceId;
use yolo_hub_terminal_protocol::PublicError;

use crate::{
    MAX_WORKSPACE_NAME_BYTES, MAX_WORKSPACE_PAGE, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
    WorkspaceAddress, validate_name,
};

pub const MAX_WORKTREE_BRANCH_BYTES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateWorktreeFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub workspace_id: Uuid,
    pub parent: WorkspaceAddress,
    pub name: String,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub trust_repository: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CreateWorktreeFields", into = "CreateWorktreeFields")]
pub struct CreateWorktreeRequest(CreateWorktreeFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveWorktreeFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub request_id: Uuid,
    pub address: WorkspaceAddress,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RemoveWorktreeFields", into = "RemoveWorktreeFields")]
pub struct RemoveWorktreeRequest(RemoveWorktreeFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListWorktreesFields {
    pub schema_version: u16,
    pub parent: WorkspaceAddress,
    pub after: Option<WorkspaceAddress>,
    pub limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ListWorktreesFields", into = "ListWorktreesFields")]
pub struct ListWorktreesRequest(ListWorktreesFields);

macro_rules! request {
    ($request:ident, $fields:ident, $validate:ident) => {
        impl $request {
            pub fn new(fields: $fields) -> Result<Self, PublicError> {
                fields.try_into()
            }
            pub fn fields(&self) -> &$fields {
                &self.0
            }
        }
        impl TryFrom<$fields> for $request {
            type Error = PublicError;
            fn try_from(fields: $fields) -> Result<Self, Self::Error> {
                if fields.schema_version != WORKSPACE_PROTOCOL_SCHEMA_VERSION {
                    return Err(PublicError::UnsupportedSchema);
                }
                let validate: fn(&$fields) -> Result<(), PublicError> = $validate;
                validate(&fields)?;
                Ok(Self(fields))
            }
        }
        impl From<$request> for $fields {
            fn from(value: $request) -> Self {
                value.0
            }
        }
    };
}

request!(CreateWorktreeRequest, CreateWorktreeFields, validate_create);
request!(RemoveWorktreeRequest, RemoveWorktreeFields, validate_remove);
request!(ListWorktreesRequest, ListWorktreesFields, validate_list);

fn validate_create(fields: &CreateWorktreeFields) -> Result<(), PublicError> {
    if fields.request_id.is_nil()
        || fields.workspace_id.is_nil()
        || fields.parent.workspace_id.as_uuid().is_nil()
        || WorkspaceId::from_uuid(fields.workspace_id).is_err()
    {
        return Err(PublicError::InvalidRequest);
    }
    validate_name(&fields.name)?;
    if fields.name.len() > MAX_WORKSPACE_NAME_BYTES {
        return Err(PublicError::InvalidMetadata);
    }
    if let Some(branch) = &fields.branch {
        validate_revision(branch)?;
    }
    if let Some(base) = &fields.base {
        validate_revision(base)?;
    }
    Ok(())
}

fn validate_remove(fields: &RemoveWorktreeFields) -> Result<(), PublicError> {
    if fields.request_id.is_nil() || fields.address.workspace_id.as_uuid().is_nil() {
        return Err(PublicError::InvalidRequest);
    }
    Ok(())
}

fn validate_list(fields: &ListWorktreesFields) -> Result<(), PublicError> {
    if fields.parent.workspace_id.as_uuid().is_nil()
        || fields.limit == 0
        || fields.limit as usize > MAX_WORKSPACE_PAGE
    {
        return Err(PublicError::InvalidRequest);
    }
    Ok(())
}

fn validate_revision(value: &str) -> Result<(), PublicError> {
    if value.is_empty()
        || value.len() > MAX_WORKTREE_BRANCH_BYTES
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("..")
        || value.contains('\0')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}
