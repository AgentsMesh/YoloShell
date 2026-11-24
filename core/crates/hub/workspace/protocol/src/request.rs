//! Workspace request envelopes and their schema validation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;

use crate::constants::{
    FIRST_WORKSPACE_REVISION, MAX_WORKSPACE_PAGE, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
};
use crate::types::{MembershipChange, WorkspaceAddress, WorkspaceSelector, WorkspaceSummary};
use crate::validation::{
    validate_labels, validate_membership, validate_name, validate_root,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkspaceFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub workspace_id: Uuid,
    pub name: String,
    pub root: String,
    pub labels: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CreateWorkspaceFields", into = "CreateWorkspaceFields")]
pub struct CreateWorkspaceRequest(CreateWorkspaceFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorkspaceFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub request_id: Uuid,
    pub address: WorkspaceAddress,
    pub expected_revision: u64,
    pub name: Option<String>,
    pub labels: Option<BTreeMap<String, String>>,
    /// Bind or unbind durable presentation members. Caller context injected
    /// at creation is not rewritten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<MembershipChange>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UpdateWorkspaceFields", into = "UpdateWorkspaceFields")]
pub struct UpdateWorkspaceRequest(UpdateWorkspaceFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListWorkspacesFields {
    pub schema_version: u16,
    pub after: Option<WorkspaceAddress>,
    pub limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ListWorkspacesFields", into = "ListWorkspacesFields")]
pub struct ListWorkspacesRequest(ListWorkspacesFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListWorkspacesResult {
    pub workspaces: Vec<WorkspaceSummary>,
    pub next: Option<WorkspaceAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceResult {
    pub workspace: WorkspaceSummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetWorkspaceFields {
    pub schema_version: u16,
    pub selector: WorkspaceSelector,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GetWorkspaceFields", into = "GetWorkspaceFields")]
pub struct GetWorkspaceRequest(GetWorkspaceFields);

impl GetWorkspaceRequest {
    pub fn new(fields: GetWorkspaceFields) -> Result<Self, PublicError> {
        fields.try_into()
    }
    pub fn fields(&self) -> &GetWorkspaceFields {
        &self.0
    }
}

impl TryFrom<GetWorkspaceFields> for GetWorkspaceRequest {
    type Error = PublicError;
    fn try_from(fields: GetWorkspaceFields) -> Result<Self, Self::Error> {
        if fields.schema_version != WORKSPACE_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        match &fields.selector {
            WorkspaceSelector::Address(address) => {
                if address.workspace_id.as_uuid().is_nil() {
                    return Err(PublicError::InvalidRequest);
                }
            }
            WorkspaceSelector::Name(name) => validate_name(name)?,
        }
        Ok(Self(fields))
    }
}

impl From<GetWorkspaceRequest> for GetWorkspaceFields {
    fn from(value: GetWorkspaceRequest) -> Self {
        value.0
    }
}

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

request!(
    CreateWorkspaceRequest,
    CreateWorkspaceFields,
    validate_create
);
request!(
    UpdateWorkspaceRequest,
    UpdateWorkspaceFields,
    validate_update
);
request!(ListWorkspacesRequest, ListWorkspacesFields, validate_list);

fn validate_create(fields: &CreateWorkspaceFields) -> Result<(), PublicError> {
    if fields.request_id.is_nil() || fields.workspace_id.is_nil() {
        return Err(PublicError::InvalidRequest);
    }
    validate_name(&fields.name)?;
    validate_root(&fields.root)?;
    validate_labels(&fields.labels)
}

fn validate_update(fields: &UpdateWorkspaceFields) -> Result<(), PublicError> {
    if fields.request_id.is_nil()
        || fields.address.workspace_id.as_uuid().is_nil()
        || fields.expected_revision < FIRST_WORKSPACE_REVISION
    {
        return Err(PublicError::InvalidRequest);
    }
    if let Some(name) = &fields.name {
        validate_name(name)?;
    }
    if let Some(labels) = &fields.labels {
        validate_labels(labels)?;
    }
    if let Some(members) = &fields.members {
        validate_membership(members)?;
    }
    Ok(())
}

fn validate_list(fields: &ListWorkspacesFields) -> Result<(), PublicError> {
    if fields.limit == 0 || fields.limit as usize > MAX_WORKSPACE_PAGE {
        return Err(PublicError::InvalidRequest);
    }
    Ok(())
}
