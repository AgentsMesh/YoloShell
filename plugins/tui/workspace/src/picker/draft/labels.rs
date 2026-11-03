use super::{Action, Draft, Kind, Request, Target};
use crate::constants::*;
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::WorkspaceAddress;
use yolo_hub_workspace_protocol::{
    UpdateWorkspaceFields, UpdateWorkspaceRequest, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
    validate_labels,
};

impl Draft {
    pub(crate) fn labels(
        address: WorkspaceAddress,
        expected_revision: u64,
        labels: &BTreeMap<String, String>,
    ) -> Self {
        let text = labels
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        Self::new(
            text,
            Target::Labels {
                address,
                expected_revision,
            },
        )
    }



    pub(crate) fn submit_labels(&mut self) -> Action {
        let labels = match parse_labels(&self.text) {
            Ok(labels) => labels,
            Err(()) => return Action::InvalidLabels,
        };
        let Target::Labels {
            address,
            expected_revision,
        } = self.target
        else {
            return Action::InvalidLabels;
        };
        let fields = UpdateWorkspaceFields {
            schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            address,
            expected_revision,
            name: None,
            labels: Some(labels),
            members: None,
        };
        match UpdateWorkspaceRequest::new(fields) {
            Ok(request) => Action::Submit(Request::Rename(request)),
            Err(_) => Action::InvalidLabels,
        }
    }
}

fn parse_labels(text: &str) -> Result<BTreeMap<String, String>, ()> {
    let mut labels = BTreeMap::new();
    let text = text.trim();
    if !text.is_empty() {
        for pair in text.split(',') {
            let (key, value) = pair.split_once('=').ok_or(())?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || labels.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(());
            }
        }
    }
    validate_labels(&labels).map_err(|_| ())?;
    Ok(labels)
}
