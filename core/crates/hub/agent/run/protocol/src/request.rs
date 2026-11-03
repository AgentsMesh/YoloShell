//! Run query and metadata request envelopes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;

use crate::constants::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, FIRST_AGENT_RUN_REVISION, MAX_AGENT_RUN_PAGE, validate_aliases,
    validate_labels, validate_name, validate_provider_id,
};
use crate::run::{AgentRunAddress, AgentRunFilter, AgentRunSelector, AgentRunSummary};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetAgentRunFields {
    pub schema_version: u16,
    pub selector: AgentRunSelector,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GetAgentRunFields", into = "GetAgentRunFields")]
pub struct GetAgentRunRequest(GetAgentRunFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListAgentRunsFields {
    pub schema_version: u16,
    pub after: Option<AgentRunAddress>,
    pub limit: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<AgentRunFilter>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ListAgentRunsFields", into = "ListAgentRunsFields")]
pub struct ListAgentRunsRequest(ListAgentRunsFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListAgentRunsResult {
    pub runs: Vec<AgentRunSummary>,
    pub next: Option<AgentRunAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAgentRunFields {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub request_id: Uuid,
    pub address: AgentRunAddress,
    pub expected_revision: u64,
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub labels: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UpdateAgentRunFields", into = "UpdateAgentRunFields")]
pub struct UpdateAgentRunRequest(UpdateAgentRunFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRunResult {
    pub run: AgentRunSummary,
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
                if fields.schema_version != AGENT_RUN_PROTOCOL_SCHEMA_VERSION {
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

request!(GetAgentRunRequest, GetAgentRunFields, validate_get);
request!(ListAgentRunsRequest, ListAgentRunsFields, validate_list);
request!(UpdateAgentRunRequest, UpdateAgentRunFields, validate_update);

fn validate_get(fields: &GetAgentRunFields) -> Result<(), PublicError> {
    crate::validate_selector(&fields.selector)
}

fn validate_list(fields: &ListAgentRunsFields) -> Result<(), PublicError> {
    if fields.limit == 0 || fields.limit as usize > MAX_AGENT_RUN_PAGE {
        return Err(PublicError::InvalidRequest);
    }
    if let Some(AgentRunFilter::Terminal { address }) = &fields.filter {
        if address.terminal_id.as_uuid().is_nil() {
            return Err(PublicError::InvalidRequest);
        }
    }
    if let Some(AgentRunFilter::Name { value }) = &fields.filter {
        validate_name(value)?;
    }
    if let Some(AgentRunFilter::Parent { address }) = &fields.filter {
        if address.run_id.as_uuid().is_nil() {
            return Err(PublicError::InvalidRequest);
        }
    }
    if let Some(AgentRunFilter::State { is }) = &fields.filter {
        if is.is_empty() || is.iter().enumerate().any(|(i, state)| is[..i].contains(state)) {
            return Err(PublicError::InvalidRequest);
        }
    }
    if let Some(AgentRunFilter::Provider { id }) = &fields.filter {
        validate_provider_id(id)?;
    }
    if let Some(AgentRunFilter::Label { key, value }) = &fields.filter {
        crate::validate_label(key, value)?;
    }
    Ok(())
}

fn validate_update(fields: &UpdateAgentRunFields) -> Result<(), PublicError> {
    if fields.request_id.is_nil()
        || fields.address.run_id.as_uuid().is_nil()
        || fields.expected_revision < FIRST_AGENT_RUN_REVISION
    {
        return Err(PublicError::InvalidRequest);
    }
    if let Some(name) = &fields.name {
        validate_name(name)?;
    }
    if let Some(aliases) = &fields.aliases {
        validate_aliases(aliases)?;
    }
    if let Some(labels) = &fields.labels {
        validate_labels(labels)?;
    }
    Ok(())
}
