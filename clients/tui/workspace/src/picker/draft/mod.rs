use crate::constants::*;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::WorkspaceAddress;
use yolo_hub_workspace_protocol::{
    CreateWorkspaceFields, CreateWorkspaceRequest, MAX_WORKSPACE_NAME_BYTES,
    UpdateWorkspaceFields, UpdateWorkspaceRequest, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
    validate_labels, validate_name,
};
use yolo_tui_line_input::Cursor;

mod labels;

pub(crate) enum Action {
    Editing,
    Cancel,
    Invalid,
    InvalidLabels,
    Submit(Request),
}

pub(crate) enum Request {
    Create(CreateWorkspaceRequest),
    Rename(UpdateWorkspaceRequest),
}

enum Target {
    Create {
        root: String,
    },
    Rename {
        address: WorkspaceAddress,
        expected_revision: u64,
    },
    Labels {
        address: WorkspaceAddress,
        expected_revision: u64,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Kind {
    Create,
    Rename,
    Labels,
}

/// One inline name draft. A create fixes the root for the draft; a rename fixes
/// the workspace revision, so neither target can drift while the user types.
pub(crate) struct Draft {
    pub(crate) text: String,
    pub(crate) cursor: Cursor,
    target: Target,
}

impl Draft {
    pub(crate) fn create(root: String, default_name: String) -> Self {
        Self::new(default_name, Target::Create { root })
    }

    pub(crate) fn rename(
        address: WorkspaceAddress,
        expected_revision: u64,
        name: String,
    ) -> Self {
        Self::new(
            name,
            Target::Rename {
                address,
                expected_revision,
            },
        )
    }

    fn new(text: String, target: Target) -> Self {
        let mut cursor = Cursor::default();
        cursor.select_all();
        Self {
            text,
            cursor,
            target,
        }
    }

    pub(crate) fn kind(&self) -> Kind {
        match self.target {
            Target::Create { .. } => Kind::Create,
            Target::Rename { .. } => Kind::Rename,
            Target::Labels { .. } => Kind::Labels,
        }
    }

    pub(crate) fn root(&self) -> Option<&str> {
        match &self.target {
            Target::Create { root } => Some(root),
            Target::Rename { .. } | Target::Labels { .. } => None,
        }
    }

    pub(crate) fn handle(&mut self, event: &Event) -> Action {
        let Event::Key(key) = event else {
            let limit = if self.kind() == Kind::Labels {
                MAX_LABEL_DRAFT_BYTES
            } else {
                MAX_WORKSPACE_NAME_BYTES
            };
            self.cursor.handle(&mut self.text, event, limit);
            return Action::Editing;
        };
        if key.kind != KeyEventKind::Press {
            return Action::Editing;
        }
        match key.code {
            KeyCode::Esc => Action::Cancel,
            KeyCode::Enter => self.submit(),
            _ => {
                let limit = if self.kind() == Kind::Labels {
                    MAX_LABEL_DRAFT_BYTES
                } else {
                    MAX_WORKSPACE_NAME_BYTES
                };
                self.cursor.handle(&mut self.text, event, limit);
                Action::Editing
            }
        }
    }

    fn submit(&mut self) -> Action {
        if self.kind() == Kind::Labels {
            return self.submit_labels();
        }
        let name = self.text.trim().to_owned();
        if name.is_empty() || validate_name(&name).is_err() {
            return Action::Invalid;
        }
        self.text = name.clone();
        match &self.target {
            Target::Create { root } => {
                if root.is_empty() {
                    return Action::Invalid;
                }
                let fields = CreateWorkspaceFields {
                    schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                    request_id: Uuid::new_v4(),
                    workspace_id: Uuid::new_v4(),
                    name,
                    root: root.clone(),
                    labels: BTreeMap::new(),
                };
                match CreateWorkspaceRequest::new(fields) {
                    Ok(request) => Action::Submit(Request::Create(request)),
                    Err(_) => Action::Invalid,
                }
            }
            Target::Rename {
                address,
                expected_revision,
            } | Target::Labels {
                address,
                expected_revision,
            } => {
                let fields = UpdateWorkspaceFields {
                    schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                    request_id: Uuid::new_v4(),
                    address: *address,
                    expected_revision: *expected_revision,
                    name: Some(name),
                    labels: None,
                    members: None,
                };
                match UpdateWorkspaceRequest::new(fields) {
                    Ok(request) => Action::Submit(Request::Rename(request)),
                    Err(_) => Action::Invalid,
                }
            }
        }
    }
}
