//! Metadata validation shared by workspace requests and persisted records.

use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_terminal_protocol::PublicError;

use crate::constants::{
    MAX_WORKSPACE_LABEL_KEY_BYTES, MAX_WORKSPACE_LABEL_VALUE_BYTES, MAX_WORKSPACE_LABELS,
    MAX_WORKSPACE_MEMBER_CHANGES, MAX_WORKSPACE_NAME_BYTES, MAX_WORKSPACE_ROOT_BYTES,
};
use crate::types::MembershipChange;

pub fn validate_name(value: &str) -> Result<(), PublicError> {
    if value.trim().is_empty() || value.len() > MAX_WORKSPACE_NAME_BYTES || value.contains('\0') {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}

pub fn validate_root(value: &str) -> Result<(), PublicError> {
    if !value.starts_with('/') || value.len() > MAX_WORKSPACE_ROOT_BYTES || value.contains('\0') {
        return Err(PublicError::InvalidMetadata);
    }
    Ok(())
}

pub fn validate_labels(labels: &BTreeMap<String, String>) -> Result<(), PublicError> {    if labels.len() > MAX_WORKSPACE_LABELS {
        return Err(PublicError::InvalidMetadata);
    }
    for (key, value) in labels {
        if key.is_empty()
            || key.len() > MAX_WORKSPACE_LABEL_KEY_BYTES
            || value.len() > MAX_WORKSPACE_LABEL_VALUE_BYTES
            || key.contains('\0')
            || value.contains('\0')
        {
            return Err(PublicError::InvalidMetadata);
        }
    }
    Ok(())
}

pub fn validate_membership(change: &MembershipChange) -> Result<(), PublicError> {
    let terminals = match change {
        MembershipChange::Bind { terminals } | MembershipChange::Unbind { terminals } => terminals,
    };
    if terminals.is_empty()
        || terminals.len() > MAX_WORKSPACE_MEMBER_CHANGES
        || terminals.iter().any(|id| id.is_nil())
        || terminals.iter().collect::<BTreeSet<_>>().len() != terminals.len()
    {
        return Err(PublicError::InvalidRequest);
    }
    Ok(())
}
