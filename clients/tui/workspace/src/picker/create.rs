use super::{Pending, Picker};
use crate::constants::*;
use crate::message::truncate;
use uuid::Uuid;

impl Picker {
    /// Applies a creation reply only while its exact request is pending.
    pub fn created(&mut self, workspace: yolo_hub_workspace_protocol::WorkspaceSummary) -> bool {
        if self.pending != Some(Pending::Create(workspace.address.workspace_id.as_uuid())) {
            return false;
        }
        self.pending = None;
        self.draft = None;
        let name = workspace.name.clone();
        let index = match self
            .rows
            .binary_search_by_key(&workspace.address, |row| row.address)
        {
            Ok(index) => {
                self.rows[index] = workspace;
                index
            }
            Err(index) => {
                self.rows.insert(index, workspace);
                index
            }
        };
        self.selected = index + 1;
        self.message = Some(format!("{CREATED}{name}"));
        true
    }

    pub fn create_failed(&mut self, workspace_id: Uuid, message: &str) {
        if self.pending != Some(Pending::Create(workspace_id)) {
            return;
        }
        self.pending = None;
        self.message = Some(truncate(message));
    }

    /// Applies a rename reply only for the exact workspace awaiting it.
    pub fn updated(
        &mut self,
        workspace: yolo_hub_workspace_protocol::WorkspaceSummary,
    ) -> bool {
        let labels = match self.pending {
            Some(Pending::Rename(address)) if address == workspace.address => false,
            Some(Pending::Labels(address)) if address == workspace.address => true,
            _ => return false,
        };
        self.pending = None;
        self.draft = None;
        let Some(index) = self
            .rows
            .iter()
            .position(|row| row.address == workspace.address)
        else {
            return false;
        };
        self.rows[index] = workspace;
        self.selected = index + 1;
        self.message = Some(if labels {
            LABELS_UPDATED.into()
        } else {
            format!("{RENAMED}{}", self.rows[index].name)
        });
        true
    }

    pub fn update_failed(
        &mut self,
        address: yolo_hub_identity::WorkspaceAddress,
        message: &str,
    ) {
        if !matches!(
            self.pending,
            Some(Pending::Rename(pending) | Pending::Labels(pending)) if pending == address
        ) {
            return;
        }
        self.pending = None;
        self.message = Some(truncate(message));
    }
}
