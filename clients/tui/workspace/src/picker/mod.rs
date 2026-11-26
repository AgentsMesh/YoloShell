mod create;
mod draft;
mod keys;

use crate::{constants::*, message::truncate};
pub(crate) use draft::{Draft, Kind};
use uuid::Uuid;
use yolo_hub_identity::{HubId, WorkspaceAddress};
use yolo_hub_workspace_protocol::{
    ListWorkspacesFields, ListWorkspacesRequest, ListWorkspacesResult, MAX_WORKSPACE_PAGE,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceSummary,
};

pub enum Handle {
    None,
    Query(ListWorkspacesRequest),
    Filter(Option<WorkspaceAddress>),
    Create(yolo_hub_workspace_protocol::CreateWorkspaceRequest),
    Rename(yolo_hub_workspace_protocol::UpdateWorkspaceRequest),
}

/// A bounded, paged list of workspaces for the current Hub. The current filter
/// is only a hint for the initial selection; the caller owns the actual scope.
#[derive(Default)]
pub struct Picker {
    visible: bool,
    loading: bool,
    hub: Option<HubId>,
    current: Option<WorkspaceAddress>,
    rows: Vec<WorkspaceSummary>,
    selected: usize,
    list_in_flight: bool,
    reopen_after_list: bool,
    message: Option<String>,
    root: Option<String>,
    draft: Option<Draft>,
    pending: Option<Pending>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Pending {
    Create(Uuid),
    Rename(WorkspaceAddress),
    Labels(WorkspaceAddress),
}

impl Picker {
    pub fn is_open(&self) -> bool {
        self.visible
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn hub(&self) -> Option<HubId> {
        self.hub
    }

    pub fn workspaces(&self) -> &[WorkspaceSummary] {
        &self.rows
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn current(&self) -> Option<WorkspaceAddress> {
        self.current
    }

    pub(crate) fn draft(&self) -> Option<&Draft> {
        self.draft.as_ref()
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Opens the picker and starts a fresh listing. A reply from a previous
    /// session is drained before the new request, so stale rows cannot leak in.
    pub fn open(
        &mut self,
        hub: HubId,
        current: Option<WorkspaceAddress>,
        root: String,
    ) -> Option<ListWorkspacesRequest> {
        self.visible = true;
        self.loading = true;
        self.hub = Some(hub);
        self.current = current;
        self.root = Some(root);
        self.draft = None;
        self.pending = None;
        self.rows.clear();
        self.selected = 0;
        self.message = None;
        if self.list_in_flight {
            self.reopen_after_list = true;
            return None;
        }
        self.list_in_flight = true;
        list_request(None)
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.loading = false;
        self.hub = None;
        self.current = None;
        self.rows.clear();
        self.selected = 0;
        self.message = None;
        self.root = None;
        self.draft = None;
        self.pending = None;
        self.reopen_after_list = false;
    }

    /// Appends one page; a next cursor yields the follow-up query.
    pub fn page(&mut self, list: ListWorkspacesResult) -> Option<ListWorkspacesRequest> {
        self.list_in_flight = false;
        if !self.visible {
            self.reopen_after_list = false;
            return None;
        }
        if self.reopen_after_list {
            self.reopen_after_list = false;
            self.rows.clear();
            self.selected = 0;
            self.loading = true;
            self.list_in_flight = true;
            return list_request(None);
        }
        self.rows.extend(list.workspaces);
        if let Some(next) = list.next {
            self.loading = true;
            self.list_in_flight = true;
            return list_request(Some(next));
        }
        self.loading = false;
        self.selected = self
            .current
            .and_then(|workspace| {
                self.rows
                    .iter()
                    .position(|row| row.address == workspace)
                    .map(|index| index + 1)
            })
            .unwrap_or(0);
        None
    }

    pub fn failed(&mut self, message: &str) {
        self.loading = false;
        self.list_in_flight = false;
        self.reopen_after_list = false;
        self.message = Some(truncate(message));
    }

}

fn list_request(after: Option<WorkspaceAddress>) -> Option<ListWorkspacesRequest> {
    ListWorkspacesRequest::new(ListWorkspacesFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        after,
        limit: MAX_WORKSPACE_PAGE as u16,
    })
    .ok()
}

